//! OS Keychain secret storage provider using platform credential stores (Keychain, Secret Service, Windows Credential Manager)

#![cfg(feature = "keyring")]

use super::envelope;
use super::{
    EncryptedSecretPayload, MemoryStorageBackend, SecretStorageMetadata, SecretStorageProvider,
    StorageBackend, StorageOptions,
};
use crate::error::{KnishIOError, Result};
use crate::storage::secure_memory::with_secure_bytes;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use rand::Rng;
use std::sync::Arc;
use zeroize::Zeroizing;

const KEY_PREFIX: &str = "knishio:secret:";
const DEFAULT_SERVICE: &str = "io.knish.secret-storage";
const DEFAULT_ALIAS: &str = "default";

/// Secret storage provider wrapping an envelope key inside the OS credential store
pub struct OsKeychainSecretStorageProvider {
    backend: Arc<dyn StorageBackend>,
    passphrase: Zeroizing<String>,
    service: String,
    alias: String,
}

impl OsKeychainSecretStorageProvider {
    /// Create a new OS keychain secret storage provider.
    ///
    /// Reads or generates a 256-bit random passphrase stored under `service` and account `knishio:kek:{alias}`.
    /// Fails closed if the platform credential store is unavailable (e.g. headless Linux without Secret Service).
    pub fn new(
        backend: Option<Arc<dyn StorageBackend>>,
        service: Option<&str>,
        alias: Option<&str>,
    ) -> Result<Self> {
        let service = service.unwrap_or(DEFAULT_SERVICE).to_string();
        let alias = alias.unwrap_or(DEFAULT_ALIAS).to_string();
        let user = format!("knishio:kek:{}", alias);

        let entry = keyring::Entry::new(&service, &user)
            .map_err(|e| KnishIOError::SecretStorage(format!("OS keychain unavailable: {e}")))?;

        let passphrase = match entry.get_password() {
            Ok(p) => Zeroizing::new(p),
            Err(keyring::Error::NoEntry) => {
                let mut random_bytes = [0u8; 32];
                rand::rng().fill_bytes(&mut random_bytes);
                let p = BASE64.encode(random_bytes);
                entry
                    .set_password(&p)
                    .map_err(|e| KnishIOError::SecretStorage(format!("OS keychain unavailable: {e}")))?;
                Zeroizing::new(p)
            }
            Err(e) => {
                return Err(KnishIOError::SecretStorage(format!(
                    "OS keychain unavailable: {e}"
                )))
            }
        };

        Ok(Self {
            backend: backend.unwrap_or_else(|| Arc::new(MemoryStorageBackend::new())),
            passphrase,
            service,
            alias,
        })
    }

    /// The service identifier used in the OS keychain
    pub fn service(&self) -> &str {
        &self.service
    }

    /// The key alias used in the OS keychain
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Execute a closure with the unwrapped secret and zeroize memory upon completion
    pub async fn with_secret<T, F>(
        &self,
        bundle_hash: &str,
        options: StorageOptions,
        f: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&str) -> Result<T> + Send,
    {
        if options.passphrase.is_some() {
            return Err(KnishIOError::SecretStorage(
                "OsKeychainSecretStorageProvider derives its passphrase from the OS keychain; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        let raw = self
            .backend
            .get_item(&format!("{KEY_PREFIX}{bundle_hash}"))?
            .ok_or_else(|| KnishIOError::SecretNotFound(bundle_hash.to_string()))?;

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {}", e)))?;

        let decrypted = envelope::open(&payload, &self.passphrase)?;

        with_secure_bytes(decrypted.to_vec(), |bytes| {
            let secret_string = String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {}", e)))?;
            f(&secret_string)
        })
    }
}

#[async_trait]
impl SecretStorageProvider for OsKeychainSecretStorageProvider {
    fn provider_type(&self) -> &str {
        "os-keychain-aes-gcm"
    }

    /// Passphrase is readable by this OS user's processes; protection is the OS login/keychain ACL,
    /// not a hardware key handle. Never claims hardware custody.
    fn is_hardware_backed(&self) -> bool {
        false
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn store_secret(
        &self,
        bundle_hash: &str,
        secret: &str,
        options: StorageOptions,
    ) -> Result<()> {
        if bundle_hash.is_empty() {
            return Err(KnishIOError::SecretStorage(
                "Bundle hash cannot be empty".to_string(),
            ));
        }
        if secret.is_empty() {
            return Err(KnishIOError::SecretStorage(
                "Secret cannot be empty".to_string(),
            ));
        }
        if options.passphrase.is_some() {
            return Err(KnishIOError::SecretStorage(
                "OsKeychainSecretStorageProvider derives its passphrase from the OS keychain; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        let metadata = SecretStorageMetadata {
            bundle_hash: bundle_hash.to_string(),
            label: options.label,
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: false,
            provider_type: "os-keychain-aes-gcm".to_string(),
        };

        let payload = envelope::seal(secret, &self.passphrase, metadata)?;
        let json_str = serde_json::to_string(&payload)
            .map_err(|e| KnishIOError::SecretStorage(format!("Serialization failed: {}", e)))?;

        self.backend
            .set_item(&format!("{KEY_PREFIX}{bundle_hash}"), json_str)?;
        Ok(())
    }

    async fn retrieve_secret(
        &self,
        bundle_hash: &str,
        options: StorageOptions,
    ) -> Result<Option<String>> {
        if options.passphrase.is_some() {
            return Err(KnishIOError::SecretStorage(
                "OsKeychainSecretStorageProvider derives its passphrase from the OS keychain; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        let raw = match self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}"))? {
            Some(val) => val,
            None => return Ok(None),
        };

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {}", e)))?;

        let decrypted = envelope::open(&payload, &self.passphrase)?;

        with_secure_bytes(decrypted.to_vec(), |bytes| {
            String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {}", e)))
        })
        .map(Some)
    }

    async fn delete_secret(&self, bundle_hash: &str) -> Result<bool> {
        self.backend
            .remove_item(&format!("{KEY_PREFIX}{bundle_hash}"))
    }

    async fn has_secret(&self, bundle_hash: &str) -> Result<bool> {
        Ok(self
            .backend
            .get_item(&format!("{KEY_PREFIX}{bundle_hash}"))?
            .is_some())
    }

    async fn list_secrets(&self) -> Result<Vec<SecretStorageMetadata>> {
        let mut results = Vec::new();
        for key in self.backend.keys()? {
            if key.starts_with(KEY_PREFIX) {
                if let Some(raw) = self.backend.get_item(&key)? {
                    if let Ok(payload) = serde_json::from_str::<EncryptedSecretPayload>(&raw) {
                        results.push(payload.metadata);
                    }
                }
            }
        }
        Ok(results)
    }
}
