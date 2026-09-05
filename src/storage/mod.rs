//! Hardware-backed envelope encryption and secure master secret storage

pub mod secure_memory;
pub mod memory;
pub mod aes_gcm;
pub use memory::MemorySecretStorageProvider;
pub use aes_gcm::AesGcmSecretStorageProvider;

use crate::error::{KnishIOError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Metadata associated with an encrypted secret in storage
///
/// WIRE FORMAT IS camelCase. This is a cross-SDK contract, not a style choice:
/// the TS, JS, and Kotlin providers all emit `bundleHash`/`createdAt`/
/// `hardwareBacked`/`providerType`, and this struct is serialized into the same
/// `EncryptedSecretPayload` JSON they read. Rust originally shipped snake_case in
/// 0.9.5, which made a TS-produced envelope fail here with
/// `missing field bundle_hash` before decryption was even attempted, and made a
/// Rust-produced envelope deserialize in TS with every typed metadata field
/// `undefined`. The `alias` attributes keep envelopes written by 0.9.5 readable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecretStorageMetadata {
    /// Bundle hash identifying the account/wallet
    #[serde(alias = "bundle_hash")]
    pub bundle_hash: String,
    /// Optional human-readable label
    pub label: Option<String>,
    /// Creation timestamp in milliseconds
    #[serde(alias = "created_at")]
    pub created_at: i64,
    /// Whether this secret is backed by hardware (TPM, Secure Enclave)
    #[serde(alias = "hardware_backed")]
    pub hardware_backed: bool,
    /// Provider type identifier
    #[serde(alias = "provider_type")]
    pub provider_type: String,
}

/// Versioned envelope encryption payload
///
/// Every field here is single-word today, so `rename_all` is currently a no-op —
/// it is declared anyway so that adding a multi-word field cannot silently
/// reintroduce the snake_case divergence described on `SecretStorageMetadata`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedSecretPayload {
    /// Payload format version (always 1)
    pub version: u32,
    /// Base64-encoded AES-GCM ciphertext
    pub ciphertext: String,
    /// Base64-encoded 12-byte initialization vector
    pub iv: String,
    /// Base64-encoded 16-byte key derivation salt
    pub salt: String,
    /// Encryption algorithm (AES-GCM)
    pub algorithm: String,
    /// PBKDF2 iteration count
    pub iterations: u32,
    /// Stored metadata
    pub metadata: SecretStorageMetadata,
}

/// Options for secret storage operations
#[derive(Debug, Clone, Default)]
pub struct StorageOptions {
    /// Optional label for the stored secret
    pub label: Option<String>,
    /// Passphrase used for PBKDF2 key derivation
    pub passphrase: Option<String>,
}

impl StorageOptions {
    /// Create storage options with a passphrase
    pub fn with_passphrase(passphrase: impl Into<String>) -> Self {
        Self {
            label: None,
            passphrase: Some(passphrase.into()),
        }
    }

    /// Create storage options with a label and passphrase
    pub fn new(label: Option<String>, passphrase: Option<String>) -> Self {
        Self { label, passphrase }
    }
}

/// Pluggable key-value persistence backend (e.g. Memory, TPM NVRAM, OS Keyring, Disk)
pub trait StorageBackend: Send + Sync {
    /// Retrieve item by key
    fn get_item(&self, key: &str) -> Option<String>;
    /// Store item by key
    fn set_item(&self, key: &str, value: String);
    /// Remove item by key, returning true if found
    fn remove_item(&self, key: &str) -> bool;
    /// List all keys
    fn keys(&self) -> Vec<String>;
}

/// Default in-memory thread-safe storage backend
pub struct MemoryStorageBackend {
    store: RwLock<HashMap<String, String>>,
}

impl MemoryStorageBackend {
    /// Create a new empty in-memory storage backend
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for MemoryStorageBackend {
    fn get_item(&self, key: &str) -> Option<String> {
        let store = self.store.read().ok()?;
        store.get(key).cloned()
    }

    fn set_item(&self, key: &str, value: String) {
        if let Ok(mut store) = self.store.write() {
            store.insert(key.to_string(), value);
        }
    }

    fn remove_item(&self, key: &str) -> bool {
        if let Ok(mut store) = self.store.write() {
            store.remove(key).is_some()
        } else {
            false
        }
    }

    fn keys(&self) -> Vec<String> {
        if let Ok(store) = self.store.read() {
            store.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

/// Contract for hardware-compatible envelope encryption secret storage providers
#[async_trait]
pub trait SecretStorageProvider: Send + Sync {
    /// Identifier of this provider type
    fn provider_type(&self) -> &str;

    /// Whether this provider is backed by hardware (TPM 2.0, Secure Enclave)
    fn is_hardware_backed(&self) -> bool;

    /// Whether this provider is available in the current environment
    async fn is_available(&self) -> bool;

    /// Store and encrypt a master secret for the given bundle hash
    async fn store_secret(&self, bundle_hash: &str, secret: &str, options: StorageOptions) -> Result<()>;

    /// Retrieve and decrypt the master secret for the given bundle hash
    async fn retrieve_secret(&self, bundle_hash: &str, options: StorageOptions) -> Result<Option<String>>;

    /// Delete a stored secret
    async fn delete_secret(&self, bundle_hash: &str) -> Result<bool>;

    /// Check if a secret exists for the given bundle hash
    async fn has_secret(&self, bundle_hash: &str) -> Result<bool>;

    /// List all stored secret metadata without exposing plaintext secrets
    async fn list_secrets(&self) -> Result<Vec<SecretStorageMetadata>>;
}

/// Execute a closure with the unwrapped secret and zeroize memory upon completion
pub async fn with_secret<P, T, F>(
    provider: &P,
    bundle_hash: &str,
    options: StorageOptions,
    f: F,
) -> Result<T>
where
    P: SecretStorageProvider + ?Sized,
    T: Send + 'static,
    F: FnOnce(&str) -> Result<T> + Send,
{
    let secret = provider
        .retrieve_secret(bundle_hash, options)
        .await?
        .ok_or_else(|| KnishIOError::SecretNotFound(bundle_hash.to_string()))?;

    crate::storage::secure_memory::with_secure_string(secret, f)
}
