//! Hardware-compatible AES-GCM envelope encryption secret storage provider
//! Uses standard AES-256-GCM with PBKDF2-HMAC-SHA256 key derivation

use super::{
    EncryptedSecretPayload, MemoryStorageBackend, SecretStorageMetadata, SecretStorageProvider,
    StorageBackend, StorageOptions,
};
use crate::error::{KnishIOError, Result};
use crate::storage::secure_memory::{with_secure_bytes, zeroize_bytes};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use rand::RngCore;
use std::sync::Arc;
use zeroize::Zeroize;

const KEY_PREFIX: &str = "knishio:secret:";
const DEFAULT_ITERATIONS: u32 = 100_000;
const SALT_LENGTH: usize = 16;
const IV_LENGTH: usize = 12;

/// AES-GCM envelope encryption secret storage provider
pub struct AesGcmSecretStorageProvider {
    backend: Arc<dyn StorageBackend>,
    default_passphrase: Option<String>,
    hardware_backed: bool,
}

impl AesGcmSecretStorageProvider {
    /// Create a new AES-GCM secret storage provider
    pub fn new(
        backend: Option<Arc<dyn StorageBackend>>,
        default_passphrase: Option<String>,
        hardware_backed: bool,
    ) -> Self {
        Self {
            backend: backend.unwrap_or_else(|| Arc::new(MemoryStorageBackend::new())),
            default_passphrase,
            hardware_backed,
        }
    }

    fn derive_key(passphrase: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
        let mut key = [0u8; 32];
        pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, iterations, &mut key);
        key
    }

    /// Execute a closure with the unwrapped secret and zeroize memory upon completion
    pub async fn with_secret<T, F>(&self, bundle_hash: &str, options: StorageOptions, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&str) -> Result<T> + Send,
    {
        let raw = self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}"))
            .ok_or_else(|| KnishIOError::SecretNotFound(bundle_hash.to_string()))?;

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {}", e)))?;

        let passphrase = options
            .passphrase
            .as_ref()
            .or(self.default_passphrase.as_ref())
            .ok_or_else(|| KnishIOError::SecretStorage("Passphrase required for secret decryption".to_string()))?;

        let salt = BASE64.decode(&payload.salt)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid salt base64: {}", e)))?;
        let iv = BASE64.decode(&payload.iv)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid IV base64: {}", e)))?;
        let ciphertext = BASE64.decode(&payload.ciphertext)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid ciphertext base64: {}", e)))?;

        let mut key = Self::derive_key(passphrase, &salt, payload.iterations);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Cipher init failed: {}", e)))?;
        key.zeroize();

        let nonce = Nonce::from_slice(&iv);
        let mut decrypted = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Decryption authentication failed: {}", e)))?;

        let secret_string = String::from_utf8(decrypted.clone())
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {}", e)))?;
        zeroize_bytes(&mut decrypted);

        let mut secret_guard = secret_string;
        let res = f(&secret_guard);
        secret_guard.zeroize();
        res
    }
}

impl Default for AesGcmSecretStorageProvider {
    fn default() -> Self {
        Self::new(None, None, false)
    }
}

#[async_trait]
impl SecretStorageProvider for AesGcmSecretStorageProvider {
    fn provider_type(&self) -> &str {
        if self.hardware_backed {
            "tpm2-aes-gcm"
        } else {
            "aes-gcm"
        }
    }

    fn is_hardware_backed(&self) -> bool {
        self.hardware_backed
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn store_secret(&self, bundle_hash: &str, secret: &str, options: StorageOptions) -> Result<()> {
        if bundle_hash.is_empty() {
            return Err(KnishIOError::SecretStorage("Bundle hash cannot be empty".to_string()));
        }
        if secret.is_empty() {
            return Err(KnishIOError::SecretStorage("Secret cannot be empty".to_string()));
        }

        let passphrase = options
            .passphrase
            .as_ref()
            .or(self.default_passphrase.as_ref())
            .ok_or_else(|| KnishIOError::SecretStorage("Passphrase required for envelope encryption".to_string()))?;

        let mut salt = [0u8; SALT_LENGTH];
        let mut iv = [0u8; IV_LENGTH];
        rand::rng().fill_bytes(&mut salt);
        rand::rng().fill_bytes(&mut iv);

        let mut key = Self::derive_key(passphrase, &salt, DEFAULT_ITERATIONS);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| KnishIOError::SecretStorage(format!("Cipher init failed: {}", e)))?;
        key.zeroize();

        let nonce = Nonce::from_slice(&iv);
        let ciphertext = cipher
            .encrypt(nonce, secret.as_bytes())
            .map_err(|e| KnishIOError::SecretStorage(format!("Encryption failed: {}", e)))?;

        let metadata = SecretStorageMetadata {
            bundle_hash: bundle_hash.to_string(),
            label: options.label,
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: self.hardware_backed,
            provider_type: self.provider_type().to_string(),
        };

        let payload = EncryptedSecretPayload {
            version: 1,
            ciphertext: BASE64.encode(&ciphertext),
            iv: BASE64.encode(iv),
            salt: BASE64.encode(salt),
            algorithm: "AES-GCM".to_string(),
            iterations: DEFAULT_ITERATIONS,
            metadata,
        };

        let json_str = serde_json::to_string(&payload)
            .map_err(|e| KnishIOError::SecretStorage(format!("Serialization failed: {}", e)))?;

        self.backend.set_item(&format!("{KEY_PREFIX}{bundle_hash}"), json_str);
        Ok(())
    }

    async fn retrieve_secret(&self, bundle_hash: &str, options: StorageOptions) -> Result<Option<String>> {
        let raw = match self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}")) {
            Some(val) => val,
            None => return Ok(None),
        };

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {}", e)))?;

        let passphrase = options
            .passphrase
            .as_ref()
            .or(self.default_passphrase.as_ref())
            .ok_or_else(|| KnishIOError::SecretStorage("Passphrase required for secret decryption".to_string()))?;

        let salt = BASE64.decode(&payload.salt)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid salt base64: {}", e)))?;
        let iv = BASE64.decode(&payload.iv)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid IV base64: {}", e)))?;
        let ciphertext = BASE64.decode(&payload.ciphertext)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid ciphertext base64: {}", e)))?;

        let mut key = Self::derive_key(passphrase, &salt, payload.iterations);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Cipher init failed: {}", e)))?;
        key.zeroize();

        let nonce = Nonce::from_slice(&iv);
        let decrypted = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Decryption authentication failed: {}", e)))?;

        with_secure_bytes(decrypted, |bytes| {
            String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {}", e)))
        }).map(Some)
    }

    async fn delete_secret(&self, bundle_hash: &str) -> Result<bool> {
        Ok(self.backend.remove_item(&format!("{KEY_PREFIX}{bundle_hash}")))
    }

    async fn has_secret(&self, bundle_hash: &str) -> Result<bool> {
        Ok(self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}")).is_some())
    }

    async fn list_secrets(&self) -> Result<Vec<SecretStorageMetadata>> {
        let mut results = Vec::new();
        for key in self.backend.keys() {
            if key.starts_with(KEY_PREFIX) {
                if let Some(raw) = self.backend.get_item(&key) {
                    if let Ok(payload) = serde_json::from_str::<EncryptedSecretPayload>(&raw) {
                        results.push(payload.metadata);
                    }
                }
            }
        }
        Ok(results)
    }
}
