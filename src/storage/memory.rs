//! Thread-safe in-memory secret storage provider
//! Used for testing, headless environments, and zero-dependency fallbacks

use super::{SecretStorageMetadata, SecretStorageProvider, StorageOptions};
use crate::error::{KnishIOError, Result};
use crate::storage::secure_memory::with_secure_string;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory secret storage provider
#[derive(Clone)]
pub struct MemorySecretStorageProvider {
    store: Arc<RwLock<HashMap<String, (String, SecretStorageMetadata)>>>,
}

impl MemorySecretStorageProvider {
    /// Create a new empty in-memory secret storage provider
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
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
            label: options.label,
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: false,
            provider_type: "memory".to_string(),
        };

        let mut store = self.store.write()
            .map_err(|e| KnishIOError::SecretStorage(format!("Lock poisoned: {}", e)))?;
        store.insert(bundle_hash.to_string(), (secret.to_string(), metadata));

        Ok(())
    }

    /// Clear all stored secrets
    pub fn clear(&self) {
        if let Ok(mut store) = self.store.write() {
            store.clear();
        }
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
}
