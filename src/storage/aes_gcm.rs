//! Software AES-GCM envelope encryption secret storage provider (never hardware-backed)
//! Uses standard AES-256-GCM with PBKDF2-HMAC-SHA256 key derivation

use super::envelope;
use super::{
    EncryptedSecretPayload, MemoryStorageBackend, SecretStorageMetadata, SecretStorageProvider,
    StorageBackend, StorageOptions, RECOVERY_KEY_PREFIX,
};
use crate::error::{KnishIOError, Result};
use crate::storage::secure_memory::with_secure_bytes;
use async_trait::async_trait;
use std::sync::Arc;
const KEY_PREFIX: &str = "knishio:secret:";


/// AES-GCM envelope encryption secret storage provider
pub struct AesGcmSecretStorageProvider {
    backend: Arc<dyn StorageBackend>,
    default_passphrase: Option<String>,
}

impl AesGcmSecretStorageProvider {
    /// Create a new AES-GCM secret storage provider
    pub fn new(
        backend: Option<Arc<dyn StorageBackend>>,
        default_passphrase: Option<String>,
    ) -> Self {
        Self {
            backend: backend.unwrap_or_else(|| Arc::new(MemoryStorageBackend::new())),
            default_passphrase,
        }
    }

    pub fn derive_key(passphrase: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
        *envelope::derive_key(passphrase, salt, iterations)
    }

    /// Execute a closure with the unwrapped secret and zeroize memory upon completion
    pub async fn with_secret<T, F>(&self, bundle_hash: &str, options: StorageOptions, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&str) -> Result<T> + Send,
    {
        let raw = self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}"))?
            .ok_or_else(|| KnishIOError::SecretNotFound(bundle_hash.to_string()))?;

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {}", e)))?;

        let passphrase = options
            .passphrase
            .as_ref()
            .or(self.default_passphrase.as_ref())
            .ok_or_else(|| KnishIOError::SecretStorage("Passphrase required for secret decryption".to_string()))?;

        let decrypted = envelope::open(&payload, passphrase)?;

        with_secure_bytes(decrypted.to_vec(), |bytes| {
            let secret_string = String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {}", e)))?;
            f(&secret_string)
        })
    }
}

impl Default for AesGcmSecretStorageProvider {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[async_trait]
impl SecretStorageProvider for AesGcmSecretStorageProvider {
    fn provider_type(&self) -> &str {
        "aes-gcm"
    }

    /// True only when this provider holds a non-exportable key inside platform-secure
    /// hardware (Android TEE/StrongBox, Secure Enclave, TPM) and learned that from the
    /// platform itself — never from a caller argument. Software envelope providers
    /// return false. The value is persisted as `metadata.hardwareBacked` in every
    /// envelope this provider writes.
    fn is_hardware_backed(&self) -> bool {
        false
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

        let metadata = SecretStorageMetadata {
            bundle_hash: bundle_hash.to_string(),
            label: options.label.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: false,
            provider_type: "aes-gcm".to_string(),
        };

        let payload = envelope::seal(secret, passphrase, metadata)?;
        let json_str = serde_json::to_string(&payload)
            .map_err(|e| KnishIOError::SecretStorage(format!("Serialization failed: {}", e)))?;

        self.backend.set_item(&format!("{KEY_PREFIX}{bundle_hash}"), json_str)?;

        if let Some(recovery_passphrase) = options.recovery_passphrase.as_ref() {
            let recovery_metadata = SecretStorageMetadata {
                bundle_hash: bundle_hash.to_string(),
                label: options.label,
                created_at: chrono::Utc::now().timestamp_millis(),
                hardware_backed: false,
                provider_type: "aes-gcm".to_string(),
            };
            let recovery_payload = envelope::seal(secret, recovery_passphrase.as_str(), recovery_metadata)?;
            let recovery_json = serde_json::to_string(&recovery_payload)
                .map_err(|e| KnishIOError::SecretStorage(format!("Serialization failed: {e}")))?;
            self.backend
                .set_item(&format!("{RECOVERY_KEY_PREFIX}{bundle_hash}"), recovery_json)?;
        }

        Ok(())
    }

    async fn retrieve_secret(&self, bundle_hash: &str, options: StorageOptions) -> Result<Option<String>> {
        let raw = match self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}"))? {
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

        let decrypted = envelope::open(&payload, passphrase)?;

        with_secure_bytes(decrypted.to_vec(), |bytes| {
            String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {}", e)))
        }).map(Some)
    }

    async fn delete_secret(&self, bundle_hash: &str) -> Result<bool> {
        let removed_primary = self.backend.remove_item(&format!("{KEY_PREFIX}{bundle_hash}"))?;
        let _ = self.backend.remove_item(&format!("{RECOVERY_KEY_PREFIX}{bundle_hash}"));
        Ok(removed_primary)
    }

    async fn has_secret(&self, bundle_hash: &str) -> Result<bool> {
        Ok(self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}"))?.is_some())
    }

    async fn list_secrets(&self) -> Result<Vec<SecretStorageMetadata>> {
        let mut results = Vec::new();
        for key in self.backend.keys()? {
            if key.starts_with(KEY_PREFIX) && !key.starts_with(RECOVERY_KEY_PREFIX) {
                if let Some(raw) = self.backend.get_item(&key)? {
                    if let Ok(payload) = serde_json::from_str::<EncryptedSecretPayload>(&raw) {
                        results.push(payload.metadata);
                    }
                }
            }
        }
        Ok(results)
    }

    async fn recover_secret(
        &self,
        bundle_hash: &str,
        recovery_passphrase: &str,
        options: StorageOptions,
    ) -> Result<()> {
        if bundle_hash.is_empty() {
            return Err(KnishIOError::SecretStorage("Bundle hash cannot be empty".to_string()));
        }
        if recovery_passphrase.is_empty() {
            return Err(KnishIOError::SecretStorage("Recovery passphrase cannot be empty".to_string()));
        }

        let raw = self
            .backend
            .get_item(&format!("{RECOVERY_KEY_PREFIX}{bundle_hash}"))?
            .ok_or_else(|| KnishIOError::SecretNotFound(format!("Recovery record not found for {bundle_hash}")))?;

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted recovery payload format: {e}")))?;

        let decrypted = envelope::open(&payload, recovery_passphrase)?;

        let secret_str = with_secure_bytes(decrypted.to_vec(), |bytes| {
            String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext in recovery envelope: {e}")))
        })?;

        let mut reenroll_options = options;
        if reenroll_options.passphrase.is_none() && self.default_passphrase.is_none() {
            reenroll_options.passphrase = Some(recovery_passphrase.to_string());
        }
        if reenroll_options.recovery_passphrase.is_none() {
            reenroll_options.recovery_passphrase = Some(zeroize::Zeroizing::new(recovery_passphrase.to_string()));
        }

        self.store_secret(bundle_hash, &secret_str, reenroll_options).await
    }
}
