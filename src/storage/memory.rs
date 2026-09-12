//! Thread-safe in-memory secret storage provider
//! Used for testing, headless environments, and zero-dependency fallbacks

use super::envelope;
use super::{EncryptedSecretPayload, SecretStorageMetadata, SecretStorageProvider, StorageOptions};
use crate::error::{KnishIOError, Result};
use crate::storage::secure_memory::{with_secure_bytes, with_secure_string};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory secret storage provider
#[derive(Clone)]
pub struct MemorySecretStorageProvider {
    store: Arc<RwLock<HashMap<String, (String, SecretStorageMetadata)>>>,
    recovery_store: Arc<RwLock<HashMap<String, String>>>,
}

impl MemorySecretStorageProvider {
    /// Create a new empty in-memory secret storage provider
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            recovery_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Synchronously store a secret in memory
    pub fn store_secret_sync(&self, bundle_hash: &str, secret: &str, options: StorageOptions) -> Result<()> {
        if bundle_hash.is_empty() {
            return Err(KnishIOError::SecretStorage("Bundle hash cannot be empty".to_string()));
        }
        if secret.is_empty() {
            return Err(KnishIOError::SecretStorage("Secret cannot be empty".to_string()));
        }

        let metadata = SecretStorageMetadata {
            bundle_hash: bundle_hash.to_string(),
            label: options.label.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: false,
            provider_type: "memory".to_string(),
        };

        {
            let mut store = self.store.write()
                .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
            store.insert(bundle_hash.to_string(), (secret.to_string(), metadata));
        }

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
            let mut recovery_store = self.recovery_store.write()
                .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {e}")))?;
            recovery_store.insert(bundle_hash.to_string(), recovery_json);
        }

        Ok(())
    }
    /// Clear all stored secrets
    pub fn clear(&self) {
        if let Ok(mut store) = self.store.write() {
            store.clear();
        }
        if let Ok(mut recovery_store) = self.recovery_store.write() {
            recovery_store.clear();
        }
    }

    /// Retrieve raw recovery envelope JSON if present
    pub fn get_recovery_payload(&self, bundle_hash: &str) -> Option<String> {
        self.recovery_store.read().ok()?.get(bundle_hash).cloned()
    }

    /// Execute a closure with the unwrapped secret and zeroize memory upon completion
    pub async fn with_secret<T, F>(&self, bundle_hash: &str, _options: StorageOptions, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&str) -> Result<T> + Send,
    {
        let secret = {
            let store = self.store.read()
                .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
            store.get(bundle_hash)
                .map(|(sec, _)| sec.clone())
                .ok_or_else(|| KnishIOError::SecretNotFound(bundle_hash.to_string()))?
        };

        with_secure_string(secret, f)
    }
}

impl Default for MemorySecretStorageProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretStorageProvider for MemorySecretStorageProvider {
    fn provider_type(&self) -> &str {
        "memory"
    }

    fn is_hardware_backed(&self) -> bool {
        false
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn store_secret(&self, bundle_hash: &str, secret: &str, options: StorageOptions) -> Result<()> {
        self.store_secret_sync(bundle_hash, secret, options)
    }

    async fn retrieve_secret(&self, bundle_hash: &str, _options: StorageOptions) -> Result<Option<String>> {
        let store = self.store.read()
            .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
        Ok(store.get(bundle_hash).map(|(sec, _)| sec.clone()))
    }

    async fn delete_secret(&self, bundle_hash: &str) -> Result<bool> {
        if let Ok(mut recovery_store) = self.recovery_store.write() {
            recovery_store.remove(bundle_hash);
        }
        let mut store = self.store.write()
            .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
        Ok(store.remove(bundle_hash).is_some())
    }

    async fn has_secret(&self, bundle_hash: &str) -> Result<bool> {
        let store = self.store.read()
            .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
        Ok(store.contains_key(bundle_hash))
    }

    async fn list_secrets(&self) -> Result<Vec<SecretStorageMetadata>> {
        let store = self.store.read()
            .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
        Ok(store.values().map(|(_, meta)| meta.clone()).collect())
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

        let raw = {
            let recovery_store = self.recovery_store.read()
                .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
            recovery_store.get(bundle_hash).cloned()
                .ok_or_else(|| KnishIOError::SecretNotFound(format!("Recovery record not found for {bundle_hash}")))?
        };

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted recovery payload format: {e}")))?;

        let decrypted = envelope::open(&payload, recovery_passphrase)?;

        let secret_str = with_secure_bytes(decrypted.to_vec(), |bytes| {
            String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext in recovery envelope: {e}")))
        })?;

        let mut reenroll_options = options;
        if reenroll_options.recovery_passphrase.is_none() {
            reenroll_options.recovery_passphrase = Some(zeroize::Zeroizing::new(recovery_passphrase.to_string()));
        }

        self.store_secret(bundle_hash, &secret_str, reenroll_options).await
    }
}
