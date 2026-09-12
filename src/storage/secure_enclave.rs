//! Apple Secure Enclave secret storage provider using hardware-bound P-256 keys (macOS only)

#![cfg(all(feature = "secure-enclave", target_os = "macos"))]

use super::envelope;
use super::{
    EncryptedSecretPayload, SecretStorageMetadata, SecretStorageProvider, StorageBackend,
    StorageOptions, RECOVERY_KEY_PREFIX,
};
use crate::error::{KnishIOError, Result};
use crate::storage::secure_memory::with_secure_bytes;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use rand::Rng;
use security_framework::item::{ItemSearchOptions, KeyClass, Location, Reference, SearchResult};
use security_framework::key::{Algorithm, GenerateKeyOptions, KeyType, SecKey, Token};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroizing;

const KEY_PREFIX: &str = "knishio:secret:";
const DEFAULT_ALIAS: &str = "default";
const LABEL_PREFIX: &str = "io.knish.secret-storage:kek:";
const RECORD_PREFIX: &str = "knishio:kek:secure-enclave:";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecureEnclaveKekRecord {
    version: u32,
    algorithm: String,
    wrapped_passphrase: String,
}

/// Secret storage provider wrapping an envelope passphrase with an Apple Secure Enclave hardware key.
pub struct SecureEnclaveSecretStorageProvider {
    backend: Arc<dyn StorageBackend>,
    passphrase: Zeroizing<String>,
    alias: String,
    key: SecKey,
}

impl SecureEnclaveSecretStorageProvider {
    /// Create a new Apple Secure Enclave secret storage provider.
    ///
    /// Reads or generates an envelope passphrase protected by a hardware-bound P-256 key
    /// stored in the Secure Enclave and referenced in the Keychain.
    ///
    /// Fails closed if the Secure Enclave or Keychain is unavailable (e.g. non-Apple hardware,
    /// or process lacks the required keychain-access-groups entitlement).
    pub fn new(
        backend: Arc<dyn StorageBackend>,
        alias: Option<&str>,
    ) -> Result<Self> {
        let alias = alias.unwrap_or(DEFAULT_ALIAS).to_string();
        let record_key = format!("{RECORD_PREFIX}{alias}");
        let label = format!("{LABEL_PREFIX}{alias}");

        let (key, passphrase) = match backend.get_item(&record_key)? {
            Some(raw) => {
                let record: SecureEnclaveKekRecord = serde_json::from_str(&raw)
                    .map_err(|e| KnishIOError::SecretStorage(format!("Corrupted Secure Enclave KEK record: {e}")))?;

                if record.wrapped_passphrase.is_empty() {
                    return Err(KnishIOError::SecretStorage("Corrupted Secure Enclave KEK record".to_string()));
                }

                // Look up key in DataProtectionKeychain by label
                let search = ItemSearchOptions::new()
                    .key_class(KeyClass::private())
                    .label(&label)
                    .load_refs(true)
                    .search()
                    .map_err(|e| {
                        KnishIOError::SecretStorage(format!("Secure Enclave unavailable: {e}"))
                    })?;

                let found_key = search.into_iter().find_map(|r| {
                    if let SearchResult::Ref(Reference::Key(k)) = r {
                        Some(k)
                    } else {
                        None
                    }
                }).ok_or_else(|| {
                    KnishIOError::SecretStorage(format!("Secure Enclave key not found in keychain for alias {alias}"))
                })?;

                let ct = BASE64.decode(&record.wrapped_passphrase)
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted wrapped passphrase base64: {e}")))?;

                let pt = found_key.decrypt_data(
                    Algorithm::ECIESEncryptionCofactorVariableIVX963SHA256AESGCM,
                    &ct,
                ).map_err(|e| KnishIOError::DecryptionFailed(format!("Secure Enclave unwrap failed: {e}")))?;

                let pass = with_secure_bytes(pt.to_vec(), |bytes| {
                    String::from_utf8(bytes.to_vec())
                        .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 passphrase: {e}")))
                })?;

                (found_key, Zeroizing::new(pass))
            }
            None => {
                // Generate a new hardware P-256 key in the Secure Enclave.
                // NOTE: SecAccessControl is deliberately omitted here. The spike proved that passing
                // any SecAccessControl (including flags=0) during SE keygen in a CLI process yields
                // errSecInteractionNotAllowed (-25308), whereas omitting it succeeds. Consequently,
                // key protection rests on the Data Protection Keychain's default accessibility
                // rather than an explicit AccessibleWhenUnlockedThisDeviceOnly + PrivateKeyUsage ACL.
                let mut gen_opts = GenerateKeyOptions::default();
                gen_opts
                    .set_key_type(KeyType::ec())
                    .set_size_in_bits(256)
                    .set_token(Token::SecureEnclave)
                    .set_location(Location::DataProtectionKeychain)
                    .set_label(&label);
                let key = SecKey::new(&gen_opts).map_err(|e| {
                    let desc = e.to_string();
                    if desc.contains("-34018") {
                        KnishIOError::SecretStorage(
                            "Secure Enclave unavailable: the process needs a keychain-access-groups entitlement (errSecMissingEntitlement); code-sign the binary or run inside an app bundle".to_string()
                        )
                    } else {
                        KnishIOError::SecretStorage(format!("Secure Enclave unavailable: {e}"))
                    }
                })?;

                let pub_key = key.public_key().ok_or_else(|| {
                    KnishIOError::SecretStorage("Failed to extract Secure Enclave public key".to_string())
                })?;

                // Generate random 32-byte passphrase
                let mut random_bytes = [0u8; 32];
                rand::rng().fill_bytes(&mut random_bytes);
                let pass = BASE64.encode(random_bytes);

                let ct = pub_key.encrypt_data(
                    Algorithm::ECIESEncryptionCofactorVariableIVX963SHA256AESGCM,
                    pass.as_bytes(),
                ).map_err(|e| KnishIOError::SecretStorage(format!("Secure Enclave encryption failed: {e}")))?;

                let record = SecureEnclaveKekRecord {
                    version: 1,
                    algorithm: "ECIES-Cofactor-VariableIV-X963-SHA256-AESGCM".to_string(),
                    wrapped_passphrase: BASE64.encode(ct),
                };

                let record_json = serde_json::to_string(&record)
                    .map_err(|e| KnishIOError::SecretStorage(format!("Serialization failed: {e}")))?;

                backend.set_item(&record_key, record_json)?;

                (key, Zeroizing::new(pass))
            }
        };

        Ok(Self {
            backend,
            passphrase,
            alias,
            key,
        })
    }

    /// The key alias used by this provider
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Unenroll this provider: deletes the Secure Enclave hardware key and removes the KEK record from backend.
    pub fn unenroll(&self) -> Result<()> {
        let _ = self.key.delete();
        let record_key = format!("{RECORD_PREFIX}{}", self.alias);
        let _ = self.backend.remove_item(&record_key);
        Ok(())
    }
}

#[async_trait]
impl SecretStorageProvider for SecureEnclaveSecretStorageProvider {
    fn provider_type(&self) -> &str {
        "secure-enclave-aes-gcm"
    }

    fn is_hardware_backed(&self) -> bool {
        true
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
            return Err(KnishIOError::SecretStorage("Bundle hash cannot be empty".to_string()));
        }
        if secret.is_empty() {
            return Err(KnishIOError::SecretStorage("Secret cannot be empty".to_string()));
        }
        if options.passphrase.is_some() {
            return Err(KnishIOError::SecretStorage(
                "SecureEnclaveSecretStorageProvider derives its passphrase from the Secure Enclave; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        if options.recovery_passphrase.is_none() && !options.allow_unrecoverable {
            return Err(KnishIOError::SecretStorage(
                "SecureEnclaveSecretStorageProvider requires recovery_passphrase unless allow_unrecoverable is true"
                    .to_string(),
            ));
        }

        let metadata = SecretStorageMetadata {
            bundle_hash: bundle_hash.to_string(),
            label: options.label.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: true,
            provider_type: "secure-enclave-aes-gcm".to_string(),
        };

        let payload = envelope::seal(secret, &self.passphrase, metadata)?;
        let json_str = serde_json::to_string(&payload)
            .map_err(|e| KnishIOError::SecretStorage(format!("Serialization failed: {e}")))?;

        self.backend
            .set_item(&format!("{KEY_PREFIX}{bundle_hash}"), json_str)?;

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

    async fn retrieve_secret(
        &self,
        bundle_hash: &str,
        options: StorageOptions,
    ) -> Result<Option<String>> {
        if options.passphrase.is_some() {
            return Err(KnishIOError::SecretStorage(
                "SecureEnclaveSecretStorageProvider derives its passphrase from the Secure Enclave; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        let raw = match self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}"))? {
            Some(val) => val,
            None => return Ok(None),
        };

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {e}")))?;

        let decrypted = envelope::open(&payload, &self.passphrase)?;

        with_secure_bytes(decrypted.to_vec(), |bytes| {
            String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {e}")))
        })
        .map(Some)
    }

    async fn delete_secret(&self, bundle_hash: &str) -> Result<bool> {
        let removed_primary = self.backend
            .remove_item(&format!("{KEY_PREFIX}{bundle_hash}"))?;
        let _ = self.backend.remove_item(&format!("{RECOVERY_KEY_PREFIX}{bundle_hash}"));
        Ok(removed_primary)
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
        if reenroll_options.recovery_passphrase.is_none() {
            reenroll_options.recovery_passphrase = Some(Zeroizing::new(recovery_passphrase.to_string()));
        }

        self.store_secret(bundle_hash, &secret_str, reenroll_options).await
    }
}
