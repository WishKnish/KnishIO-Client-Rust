//! TPM 2.0 hardware-enclave secret storage provider
//!
//! Uses a deterministic ECC P-256 primary key under the Owner hierarchy
//! to seal a random 256-bit device passphrase into a TPM KeyedHash object.
//! Evaluates hardware custody from platform TPM properties.

#![cfg(feature = "tpm")]

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
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tss_esapi::attributes::ObjectAttributesBuilder;
use tss_esapi::interface_types::algorithm::{HashingAlgorithm, PublicAlgorithm};
use tss_esapi::constants::PropertyTag;
use tss_esapi::interface_types::ecc::EccCurve;
use tss_esapi::interface_types::resource_handles::Hierarchy;
use tss_esapi::interface_types::session_handles::AuthSession;
use tss_esapi::structures::{
    EccPoint, EccScheme, KeyDerivationFunctionScheme, KeyedHashScheme, Private, Public,
    PublicBuilder, PublicEccParametersBuilder, PublicKeyedHashParameters, SensitiveData,
    SymmetricDefinitionObject,
};
use tss_esapi::tcti_ldr::TctiNameConf;
use tss_esapi::traits::{Marshall, UnMarshall};
use tss_esapi::Context;
use zeroize::Zeroizing;

const KEY_PREFIX: &str = "knishio:secret:";
const DEFAULT_ALIAS: &str = "default";
const DEFAULT_DEVICE_TCTI: &str = "device:/dev/tpmrm0";

/// Check if the connected TPM is a software emulator (swtpm/libtpms)
fn is_software_tpm(context: &mut Context) -> bool {
    let manufacturer = context
        .get_tpm_property(PropertyTag::Manufacturer)
        .ok()
        .flatten()
        .unwrap_or(0);
    let vendor1 = context
        .get_tpm_property(PropertyTag::VendorString1)
        .ok()
        .flatten()
        .unwrap_or(0);

    let m_bytes = manufacturer.to_be_bytes();
    let v_bytes = vendor1.to_be_bytes();

    &m_bytes == b"IBM " && v_bytes.starts_with(b"SW")
}

/// Deterministic ECC P-256 primary key template under Owner hierarchy (TCG storage key standard)
fn build_primary_template() -> Result<Public> {
    let obj_attrs = ObjectAttributesBuilder::new()
        .with_fixed_tpm(true)
        .with_fixed_parent(true)
        .with_sensitive_data_origin(true)
        .with_user_with_auth(true)
        .with_restricted(true)
        .with_decrypt(true)
        .build()
        .map_err(|e| KnishIOError::SecretStorage(format!("Primary ObjectAttributes failed: {e}")))?;

    let ecc_params = PublicEccParametersBuilder::new()
        .with_ecc_scheme(EccScheme::Null)
        .with_curve(EccCurve::NistP256)
        .with_symmetric(SymmetricDefinitionObject::AES_128_CFB)
        .with_key_derivation_function_scheme(KeyDerivationFunctionScheme::Null)
        .with_is_decryption_key(true)
        .with_restricted(true)
        .build()
        .map_err(|e| KnishIOError::SecretStorage(format!("Primary EccParams failed: {e}")))?;

    PublicBuilder::new()
        .with_public_algorithm(PublicAlgorithm::Ecc)
        .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
        .with_object_attributes(obj_attrs)
        .with_ecc_parameters(ecc_params)
        .with_ecc_unique_identifier(EccPoint::default())
        .build()
        .map_err(|e| KnishIOError::SecretStorage(format!("Primary PublicBuilder failed: {e}")))
}

/// KeyedHash template for sealing the device passphrase
fn build_keyed_hash_template() -> Result<Public> {
    let obj_attrs = ObjectAttributesBuilder::new()
        .with_fixed_tpm(true)
        .with_fixed_parent(true)
        .with_user_with_auth(true)
        .build()
        .map_err(|e| KnishIOError::SecretStorage(format!("KeyedHash ObjectAttributes failed: {e}")))?;

    PublicBuilder::new()
        .with_public_algorithm(PublicAlgorithm::KeyedHash)
        .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
        .with_object_attributes(obj_attrs)
        .with_auth_policy(Default::default())
        .with_keyed_hash_parameters(PublicKeyedHashParameters::new(KeyedHashScheme::Null))
        .with_keyed_hash_unique_identifier(Default::default())
        .build()
        .map_err(|e| KnishIOError::SecretStorage(format!("KeyedHash PublicBuilder failed: {e}")))
}

/// Secret storage provider backed by a TPM 2.0 sealed device key
pub struct Tpm2SecretStorageProvider {
    backend: Arc<dyn StorageBackend>,
    context: Mutex<Context>,
    alias: String,
    hardware_backed: bool,
}

impl Tpm2SecretStorageProvider {
    /// Create a new TPM 2.0 secret storage provider.
    ///
    /// TCTI is resolved from `tcti` argument, `KNISHIO_TPM_TCTI` environment variable, or defaults to `device:/dev/tpmrm0`.
    /// Fails closed if the TPM or TCTI interface is unavailable.
    pub fn new(
        backend: Option<Arc<dyn StorageBackend>>,
        tcti: Option<&str>,
        alias: Option<&str>,
    ) -> Result<Self> {
        let tcti_str = match tcti {
            Some(t) => t.to_string(),
            None => std::env::var("KNISHIO_TPM_TCTI")
                .unwrap_or_else(|_| DEFAULT_DEVICE_TCTI.to_string()),
        };

        let tcti_conf = TctiNameConf::from_str(&tcti_str)
            .map_err(|e| KnishIOError::SecretStorage(format!("TPM unavailable: {e}")))?;

        let mut context = Context::new(tcti_conf)
            .map_err(|e| KnishIOError::SecretStorage(format!("TPM unavailable: {e}")))?;

        let hardware_backed = !is_software_tpm(&mut context);
        let alias = alias.unwrap_or(DEFAULT_ALIAS).to_string();

        Ok(Self {
            backend: backend.unwrap_or_else(|| Arc::new(MemoryStorageBackend::new())),
            context: Mutex::new(context),
            alias,
            hardware_backed,
        })
    }

    /// Retrieve or generate and seal the device passphrase in TPM
    fn get_or_create_passphrase(&self) -> Result<Zeroizing<String>> {
        let mut context = self
            .context
            .lock()
            .map_err(|_| KnishIOError::SecretStorage("TPM context lock poisoned".into()))?;

        context.set_sessions((Some(AuthSession::Password), None, None));

        let record_key = format!("knishio:kek:tpm2:{}", self.alias);
        let raw_record = self.backend.get_item(&record_key)?;

        match raw_record {
            None => {
                // First use: create primary, generate passphrase, seal into KeyedHash
                let primary_template = build_primary_template()?;
                let primary_res = context
                    .create_primary(Hierarchy::Owner, primary_template, None, None, None, None)
                    .map_err(|e| KnishIOError::SecretStorage(format!("TPM create_primary failed: {e}")))?;
                let primary_handle = primary_res.key_handle;

                let mut random_bytes = [0u8; 32];
                rand::rng().fill_bytes(&mut random_bytes);
                let passphrase = BASE64.encode(random_bytes);

                let sensitive = SensitiveData::try_from(passphrase.as_bytes().to_vec())
                    .map_err(|e| KnishIOError::SecretStorage(format!("SensitiveData failed: {e}")))?;

                let keyed_hash_template = build_keyed_hash_template()?;
                let create_res = context.create(
                    primary_handle,
                    keyed_hash_template,
                    None,
                    Some(sensitive),
                    None,
                    None,
                );

                let _ = context.flush_context(primary_handle.into());

                let create_res = create_res
                    .map_err(|e| KnishIOError::SecretStorage(format!("TPM create sealed object failed: {e}")))?;

                let pub_b64 = BASE64.encode(
                    create_res
                        .out_public
                        .marshall()
                        .map_err(|e| KnishIOError::SecretStorage(format!("Marshall public failed: {e}")))?,
                );
                let priv_b64 = BASE64.encode(create_res.out_private.value());

                let record = serde_json::json!({
                    "version": 1,
                    "public": pub_b64,
                    "private": priv_b64,
                });

                self.backend
                    .set_item(&record_key, serde_json::to_string(&record)?)?;

                Ok(Zeroizing::new(passphrase))
            }
            Some(raw) => {
                // Subsequent use: load sealed object under deterministic primary, unseal
                let json: serde_json::Value = serde_json::from_str(&raw)
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted TPM record format: {e}")))?;

                let pub_b64 = json["public"]
                    .as_str()
                    .ok_or_else(|| KnishIOError::DecryptionFailed("Missing public in TPM record".into()))?;
                let priv_b64 = json["private"]
                    .as_str()
                    .ok_or_else(|| KnishIOError::DecryptionFailed("Missing private in TPM record".into()))?;

                let pub_bytes = BASE64
                    .decode(pub_b64)
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid public base64: {e}")))?;
                let priv_bytes = BASE64
                    .decode(priv_b64)
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid private base64: {e}")))?;

                let public = Public::unmarshall(&pub_bytes)
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted public TPM data: {e}")))?;
                let private = Private::try_from(priv_bytes)
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted private TPM data: {e}")))?;

                let primary_template = build_primary_template()?;
                let primary_res = context
                    .create_primary(Hierarchy::Owner, primary_template, None, None, None, None)
                    .map_err(|e| KnishIOError::SecretStorage(format!("TPM create_primary failed: {e}")))?;
                let primary_handle = primary_res.key_handle;

                let load_res = context.load(primary_handle, private, public);
                let _ = context.flush_context(primary_handle.into());

                let loaded_handle = load_res
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("TPM load sealed object failed: {e}")))?;

                let unseal_res = context.unseal(loaded_handle.into());
                let _ = context.flush_context(loaded_handle.into());

                let unsealed = unseal_res
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("TPM unseal failed: {e}")))?;

                let passphrase_str = String::from_utf8(unsealed.value().to_vec())
                    .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 passphrase from TPM: {e}")))?;

                Ok(Zeroizing::new(passphrase_str))
            }
        }
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
                "Tpm2SecretStorageProvider derives its passphrase from the TPM-sealed device key; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        let passphrase = self.get_or_create_passphrase()?;

        let raw = self
            .backend
            .get_item(&format!("{KEY_PREFIX}{bundle_hash}"))?
            .ok_or_else(|| KnishIOError::SecretNotFound(bundle_hash.to_string()))?;

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {}", e)))?;

        let decrypted = envelope::open(&payload, &passphrase)?;

        with_secure_bytes(decrypted.to_vec(), |bytes| {
            let secret_string = String::from_utf8(bytes.to_vec())
                .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid UTF-8 plaintext: {}", e)))?;
            f(&secret_string)
        })
    }
}

#[async_trait]
impl SecretStorageProvider for Tpm2SecretStorageProvider {
    fn provider_type(&self) -> &str {
        "tpm2-aes-gcm"
    }

    fn is_hardware_backed(&self) -> bool {
        self.hardware_backed
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
                "Tpm2SecretStorageProvider derives its passphrase from the TPM-sealed device key; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        let passphrase = self.get_or_create_passphrase()?;

        let metadata = SecretStorageMetadata {
            bundle_hash: bundle_hash.to_string(),
            label: options.label,
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: self.hardware_backed,
            provider_type: "tpm2-aes-gcm".to_string(),
        };

        let payload = envelope::seal(secret, &passphrase, metadata)?;
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
                "Tpm2SecretStorageProvider derives its passphrase from the TPM-sealed device key; StorageOptions.passphrase is not accepted"
                    .to_string(),
            ));
        }

        let raw = match self.backend.get_item(&format!("{KEY_PREFIX}{bundle_hash}"))? {
            Some(val) => val,
            None => return Ok(None),
        };

        let payload: EncryptedSecretPayload = serde_json::from_str(&raw)
            .map_err(|e| KnishIOError::DecryptionFailed(format!("Corrupted payload format: {}", e)))?;

        let passphrase = self.get_or_create_passphrase()?;
        let decrypted = envelope::open(&payload, &passphrase)?;

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
