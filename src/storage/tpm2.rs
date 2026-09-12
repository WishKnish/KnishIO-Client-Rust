//! TPM 2.0 hardware-enclave secret storage provider
//!
//! Uses a deterministic ECC P-256 primary key under the Owner hierarchy
//! to seal a random 256-bit device passphrase into a TPM KeyedHash object.
//! Evaluates hardware custody from platform TPM properties.

#![cfg(feature = "tpm")]

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
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tss_esapi::attributes::{ObjectAttributesBuilder, SessionAttributesBuilder};
use tss_esapi::constants::{PropertyTag, SessionType};
use tss_esapi::interface_types::algorithm::{HashingAlgorithm, PublicAlgorithm};
use tss_esapi::interface_types::ecc::EccCurve;
use tss_esapi::interface_types::resource_handles::Hierarchy;
use tss_esapi::handles::SessionHandle;
use tss_esapi::interface_types::session_handles::{AuthSession, PolicySession};
use tss_esapi::structures::{
    Auth, Digest, EccPoint, EccScheme, KeyDerivationFunctionScheme, KeyedHashScheme,
    PcrSelectionList, PcrSelectionListBuilder, PcrSlot, Private, Public, PublicBuilder,
    PublicEccParametersBuilder, PublicKeyedHashParameters, SensitiveData, SymmetricDefinition,
    SymmetricDefinitionObject,
};
use tss_esapi::tcti_ldr::TctiNameConf;
use tss_esapi::traits::{Marshall, UnMarshall};
use tss_esapi::Context;
use zeroize::Zeroizing;
const KEY_PREFIX: &str = "knishio:secret:";
const DEFAULT_ALIAS: &str = "default";
const DEFAULT_DEVICE_TCTI: &str = "device:/dev/tpmrm0";

/// Classification of TPM hardware backing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmIdentity {
    /// Physical TPM hardware module
    Hardware,
    /// Known software emulator (swtpm, libtpms, Microsoft simulator)
    Software,
    /// Unrecognized or unreadable TPM identity
    Unknown,
}

/// TPM 2.0 PCR authorization policy configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tpm2Policy {
    /// PCR hashing algorithm bank (e.g. HashingAlgorithm::Sha256)
    pub pcr_bank: HashingAlgorithm,
    /// PCR slot indices to bind (e.g. vec![7])
    pub pcrs: Vec<u8>,
    /// Optional authorization password
    pub auth: Option<Zeroizing<String>>,
}

impl Default for Tpm2Policy {
    fn default() -> Self {
        Self {
            pcr_bank: HashingAlgorithm::Sha256,
            pcrs: vec![7],
            auth: None,
        }
    }
}

impl Tpm2Policy {
    /// Create a new TPM 2.0 PCR policy
    pub fn new(pcr_bank: HashingAlgorithm, pcrs: Vec<u8>, auth: Option<Zeroizing<String>>) -> Self {
        Self { pcr_bank, pcrs, auth }
    }

    /// Create a new TPM 2.0 PCR policy with specified PCR slots
    pub fn with_pcrs(pcr_bank: HashingAlgorithm, pcrs: Vec<u8>) -> Self {
        Self { pcr_bank, pcrs, auth: None }
    }
}

/// Build a PcrSelectionList from a bank and PCR indices
pub fn build_pcr_selection_list(pcr_bank: HashingAlgorithm, pcrs: &[u8]) -> Result<PcrSelectionList> {
    let mut slots = Vec::new();
    for &pcr in pcrs {
        if pcr >= 32 {
            return Err(KnishIOError::SecretStorage(format!(
                "Invalid PCR index: {pcr} (must be 0..31)"
            )));
        }
        let slot_mask = 1u32
            .checked_shl(pcr as u32)
            .ok_or_else(|| KnishIOError::SecretStorage(format!("Invalid PCR slot: {pcr}")))?;
        let slot = PcrSlot::try_from(slot_mask)
            .map_err(|e| KnishIOError::SecretStorage(format!("Invalid PCR slot {pcr}: {e}")))?;
        slots.push(slot);
    }
    PcrSelectionListBuilder::new()
        .with_selection(pcr_bank, &slots)
        .build()
        .map_err(|e| KnishIOError::SecretStorage(format!("Failed to build PcrSelectionList: {e}")))
}

/// Classify TPM identity from manufacturer ID and vendor string words
pub fn classify_tpm_identity(manufacturer: u32, vendor_strings: [u32; 4]) -> TpmIdentity {
    if manufacturer == 0 {
        return TpmIdentity::Unknown;
    }

    let m_bytes = manufacturer.to_be_bytes();
    let mut vendor_bytes = Vec::with_capacity(16);
    for v in vendor_strings {
        if v != 0 {
            for b in v.to_be_bytes() {
                if b != 0 {
                    vendor_bytes.push(b);
                }
            }
        }
    }
    let vendor_str = String::from_utf8_lossy(&vendor_bytes);
    let vendor_upper = vendor_str.to_ascii_uppercase();

    // 1. Known software emulator signatures (fail-closed check):
    // - swtpm / libtpms: Manufacturer "IBM " and VendorString starts with "SW"
    // - Microsoft simulator: Manufacturer "MSFT" with simulator strings or vendor strings containing "SIMULATOR"
    // - Generic emulator signatures containing "EMULAT", "SWTPM", "VTPM", or "QEMU"
    let is_swtpm = &m_bytes == b"IBM "
        && (vendor_strings[0].to_be_bytes().starts_with(b"SW") || vendor_upper.starts_with("SW"));

    let is_simulator = &m_bytes == b"MSFT"
        || vendor_upper.contains("MSFT")
        || vendor_upper.contains("SIMULATOR")
        || vendor_upper.contains("EMULAT")
        || vendor_upper.contains("SWTPM")
        || vendor_upper.contains("VTPM")
        || vendor_upper.contains("QEMU");

    if is_swtpm || is_simulator {
        return TpmIdentity::Software;
    }

    // 2. Strict allowlist of known physical TPM hardware manufacturers (TCG Vendor ID Registry)
    const KNOWN_HARDWARE_MFGS: &[[u8; 4]] = &[
        *b"AMD ", // AMD firmware TPM
        *b"ATML", // Atmel / Microchip
        *b"BRCM", // Broadcom
        *b"HPE ", // HP Enterprise
        *b"IFX ", // Infineon Technologies
        *b"INTC", // Intel Platform Trust Technology (PTT)
        *b"LEN ", // Lenovo
        *b"NTC ", // Nuvoton Technology
        *b"NTZ ", // Nationz Technologies
        *b"QCOM", // Qualcomm
        *b"SMSC", // SMSC
        *b"STM ", // STMicroelectronics
        *b"TXN ", // Texas Instruments
    ];

    if KNOWN_HARDWARE_MFGS.contains(&m_bytes) {
        TpmIdentity::Hardware
    } else {
        // Unrecognized or third-party manufacturer: fail closed to Unknown (never claims Hardware)
        TpmIdentity::Unknown
    }
}

/// Check TPM identity from Context properties (TCG standard properties)
pub fn tpm_identity(context: &mut Context) -> TpmIdentity {
    let manufacturer = match context.get_tpm_property(PropertyTag::Manufacturer) {
        Ok(Some(m)) if m != 0 => m,
        _ => return TpmIdentity::Unknown,
    };

    let vendor1 = match context.get_tpm_property(PropertyTag::VendorString1) {
        Ok(v) => v.unwrap_or(0),
        Err(_) => return TpmIdentity::Unknown,
    };
    let vendor2 = match context.get_tpm_property(PropertyTag::VendorString2) {
        Ok(v) => v.unwrap_or(0),
        Err(_) => return TpmIdentity::Unknown,
    };
    let vendor3 = match context.get_tpm_property(PropertyTag::VendorString3) {
        Ok(v) => v.unwrap_or(0),
        Err(_) => return TpmIdentity::Unknown,
    };
    let vendor4 = match context.get_tpm_property(PropertyTag::VendorString4) {
        Ok(v) => v.unwrap_or(0),
        Err(_) => return TpmIdentity::Unknown,
    };

    classify_tpm_identity(manufacturer, [vendor1, vendor2, vendor3, vendor4])
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
fn build_keyed_hash_template(policy_digest: Option<Digest>, has_auth: bool) -> Result<Public> {
    let obj_attrs = ObjectAttributesBuilder::new()
        .with_fixed_tpm(true)
        .with_fixed_parent(true)
        .with_user_with_auth(policy_digest.is_none() || has_auth)
        .build()
        .map_err(|e| KnishIOError::SecretStorage(format!("KeyedHash ObjectAttributes failed: {e}")))?;

    let mut builder = PublicBuilder::new()
        .with_public_algorithm(PublicAlgorithm::KeyedHash)
        .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
        .with_object_attributes(obj_attrs);

    if let Some(digest) = policy_digest {
        builder = builder.with_auth_policy(digest);
    } else {
        builder = builder.with_auth_policy(Default::default());
    }

    builder
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
    policy: Option<Tpm2Policy>,
}

impl Tpm2SecretStorageProvider {
    /// Create a new TPM 2.0 secret storage provider.
    ///
    /// TCTI is resolved from `tcti` argument, `KNISHIO_TPM_TCTI` environment variable, or defaults to `device:/dev/tpmrm0`.
    /// Fails closed if the TPM or TCTI interface is unavailable.
    ///
    /// The KEK is durable; the backend must be too. Callers must supply an explicit storage backend
    /// rather than relying on an ephemeral in-memory default.
    pub fn new(
        backend: Arc<dyn StorageBackend>,
        tcti: Option<&str>,
        alias: Option<&str>,
        policy: Option<Tpm2Policy>,
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

        let identity = tpm_identity(&mut context);
        let hardware_backed = matches!(identity, TpmIdentity::Hardware);
        let alias = alias.unwrap_or(DEFAULT_ALIAS).to_string();

        Ok(Self {
            backend,
            context: Mutex::new(context),
            alias,
            hardware_backed,
            policy,
        })
    }

    /// Active TPM policy if configured
    pub fn policy(&self) -> Option<&Tpm2Policy> {
        self.policy.as_ref()
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

                let (keyed_hash_template, auth_value) = if let Some(policy) = &self.policy {
                    let pcrs = if policy.pcrs.is_empty() {
                        &[7]
                    } else {
                        policy.pcrs.as_slice()
                    };
                    let pcr_selection_list = build_pcr_selection_list(policy.pcr_bank, pcrs)?;

                    let policy_digest = context.execute_without_session(|ctx| -> Result<Digest> {
                        let trial_auth_session = ctx
                            .start_auth_session(
                                None,
                                None,
                                None,
                                SessionType::Trial,
                                SymmetricDefinition::AES_256_CFB,
                                policy.pcr_bank,
                            )
                            .map_err(|e| KnishIOError::SecretStorage(format!("Start trial auth session failed: {e}")))?
                            .ok_or_else(|| KnishIOError::SecretStorage("Start trial auth session returned None".into()))?;

                        let (trial_attrs, trial_attrs_mask) = SessionAttributesBuilder::new()
                            .with_decrypt(true)
                            .with_encrypt(true)
                            .build();
                        let _ = ctx.tr_sess_set_attributes(trial_auth_session, trial_attrs, trial_attrs_mask);

                        let trial_policy_session = PolicySession::try_from(trial_auth_session)
                            .map_err(|e| KnishIOError::SecretStorage(format!("Convert to PolicySession failed: {e}")))?;

                        ctx.policy_pcr(trial_policy_session, Digest::default(), pcr_selection_list)
                            .map_err(|e| KnishIOError::SecretStorage(format!("Trial policy_pcr failed: {e}")))?;

                        let digest = ctx
                            .policy_get_digest(trial_policy_session)
                            .map_err(|e| KnishIOError::SecretStorage(format!("Trial policy_get_digest failed: {e}")))?;

                        let _ = ctx.flush_context(SessionHandle::from(trial_policy_session).into());

                        Ok(digest)
                    })?;

                    let auth = policy.auth.as_ref().map(|a| Auth::try_from(a.as_bytes().to_vec())).transpose()
                        .map_err(|e| KnishIOError::SecretStorage(format!("Invalid auth value: {e}")))?;

                    (build_keyed_hash_template(Some(policy_digest), auth.is_some())?, auth)
                } else {
                    (build_keyed_hash_template(None, false)?, None)
                };

                let create_res = context.create(
                    primary_handle,
                    keyed_hash_template,
                    auth_value,
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

                let mut record = serde_json::json!({
                    "version": 1,
                    "public": pub_b64,
                    "private": priv_b64,
                });

                if let Some(policy) = &self.policy {
                    let bank_str = match policy.pcr_bank {
                        HashingAlgorithm::Sha256 => "sha256",
                        HashingAlgorithm::Sha1 => "sha1",
                        HashingAlgorithm::Sha384 => "sha384",
                        HashingAlgorithm::Sha512 => "sha512",
                        _ => "sha256",
                    };
                    let pcrs = if policy.pcrs.is_empty() {
                        vec![7]
                    } else {
                        policy.pcrs.clone()
                    };
                    record["policy"] = serde_json::json!({
                        "bank": bank_str,
                        "pcrs": pcrs,
                    });
                }

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

                let policy_info = json.get("policy");
                let unsealed = if let Some(p_val) = policy_info {
                    let bank = match p_val.get("bank").and_then(|b| b.as_str()) {
                        Some("sha256") => HashingAlgorithm::Sha256,
                        Some("sha1") => HashingAlgorithm::Sha1,
                        Some("sha384") => HashingAlgorithm::Sha384,
                        Some("sha512") => HashingAlgorithm::Sha512,
                        _ => HashingAlgorithm::Sha256,
                    };
                    let pcrs: Vec<u8> = p_val
                        .get("pcrs")
                        .and_then(|p| p.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
                        .unwrap_or_else(|| vec![7]);

                    let pcr_selection_list = build_pcr_selection_list(bank, &pcrs)?;

                    let policy_session = context.execute_without_session(|ctx| -> Result<PolicySession> {
                        let auth_session = ctx
                            .start_auth_session(
                                None,
                                None,
                                None,
                                SessionType::Policy,
                                SymmetricDefinition::AES_256_CFB,
                                bank,
                            )
                            .map_err(|e| KnishIOError::SecretStorage(format!("Start policy session failed: {e}")))?
                            .ok_or_else(|| KnishIOError::SecretStorage("Start policy session returned None".into()))?;

                        let (session_attrs, session_attrs_mask) = SessionAttributesBuilder::new()
                            .with_decrypt(true)
                            .with_encrypt(true)
                            .build();
                        let _ = ctx.tr_sess_set_attributes(auth_session, session_attrs, session_attrs_mask);

                        let policy_session = PolicySession::try_from(auth_session)
                            .map_err(|e| KnishIOError::SecretStorage(format!("Convert to PolicySession failed: {e}")))?;

                        ctx.policy_pcr(policy_session, Digest::default(), pcr_selection_list)
                            .map_err(|e| KnishIOError::DecryptionFailed(format!("TPM policy_pcr failed: {e}")))?;

                        Ok(policy_session)
                    });

                    let policy_session = match policy_session {
                        Ok(ps) => ps,
                        Err(e) => {
                            let _ = context.flush_context(loaded_handle.into());
                            return Err(e);
                        }
                    };

                    context.set_sessions((Some(AuthSession::PolicySession(policy_session)), None, None));

                    if let Some(policy) = &self.policy {
                        if let Some(auth_str) = &policy.auth {
                            if let Ok(auth_val) = Auth::try_from(auth_str.as_bytes().to_vec()) {
                                let _ = context.tr_set_auth(loaded_handle.into(), auth_val);
                            }
                        }
                    }

                    let unseal_res = context.unseal(loaded_handle.into());
                    let _ = context.flush_context(loaded_handle.into());
                    let _ = context.flush_context(SessionHandle::from(policy_session).into());
                    context.set_sessions((Some(AuthSession::Password), None, None));

                    unseal_res.map_err(|e| KnishIOError::DecryptionFailed(format!("TPM unseal failed: {e}")))?
                } else {
                    let unseal_res = context.unseal(loaded_handle.into());
                    let _ = context.flush_context(loaded_handle.into());
                    unseal_res.map_err(|e| KnishIOError::DecryptionFailed(format!("TPM unseal failed: {e}")))?
                };
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
        if self.policy.is_some() && options.recovery_passphrase.is_none() && !options.allow_unrecoverable {
            return Err(KnishIOError::SecretStorage(
                "Tpm2SecretStorageProvider with PCR policy requires recovery_passphrase unless allow_unrecoverable is true"
                    .to_string(),
            ));
        }

        let passphrase = self.get_or_create_passphrase()?;

        let metadata = SecretStorageMetadata {
            bundle_hash: bundle_hash.to_string(),
            label: options.label.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
            hardware_backed: self.hardware_backed,
            provider_type: "tpm2-aes-gcm".to_string(),
        };

        let payload = envelope::seal(secret, &passphrase, metadata)?;
        let json_str = serde_json::to_string(&payload)
            .map_err(|e| KnishIOError::SecretStorage(format!("Serialization failed: {}", e)))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_tpm_identity_software_swtpm() {
        let ibm = u32::from_be_bytes(*b"IBM ");
        let sw0 = u32::from_be_bytes(*b"SW\0\0");
        assert_eq!(classify_tpm_identity(ibm, [sw0, 0, 0, 0]), TpmIdentity::Software);

        let swtpm = u32::from_be_bytes(*b"SWTP");
        assert_eq!(classify_tpm_identity(ibm, [swtpm, 0, 0, 0]), TpmIdentity::Software);
    }

    #[test]
    fn test_classify_tpm_identity_software_msft() {
        let msft = u32::from_be_bytes(*b"MSFT");
        assert_eq!(classify_tpm_identity(msft, [0, 0, 0, 0]), TpmIdentity::Software);

        let other_mfg = u32::from_be_bytes(*b"XXXX");
        let sim1 = u32::from_be_bytes(*b"SIMU");
        let sim2 = u32::from_be_bytes(*b"LATO");
        let sim3 = u32::from_be_bytes(*b"R\0\0\0");
        assert_eq!(
            classify_tpm_identity(other_mfg, [sim1, sim2, sim3, 0]),
            TpmIdentity::Software
        );
    }

    #[test]
    fn test_classify_tpm_identity_hardware() {
        let ifx = u32::from_be_bytes(*b"IFX ");
        assert_eq!(classify_tpm_identity(ifx, [0, 0, 0, 0]), TpmIdentity::Hardware);

        let ntc = u32::from_be_bytes(*b"NTC ");
        assert_eq!(classify_tpm_identity(ntc, [0, 0, 0, 0]), TpmIdentity::Hardware);

        let stm = u32::from_be_bytes(*b"STM ");
        assert_eq!(classify_tpm_identity(stm, [0, 0, 0, 0]), TpmIdentity::Hardware);
    }

    #[test]
    fn test_classify_tpm_identity_unknown_fails_closed() {
        assert_eq!(classify_tpm_identity(0, [0, 0, 0, 0]), TpmIdentity::Unknown);

        // Unrecognized manufacturer ID must return Unknown, NOT Hardware (fail closed allowlist)
        let rand_mfg = u32::from_be_bytes(*b"RAND");
        assert_eq!(classify_tpm_identity(rand_mfg, [0, 0, 0, 0]), TpmIdentity::Unknown);

        let unknown_mfg = u32::from_be_bytes(*b"UNKN");
        assert_eq!(classify_tpm_identity(unknown_mfg, [0, 0, 0, 0]), TpmIdentity::Unknown);

        // Fail-closed verification: Unknown does NOT claim hardware
        assert!(!matches!(TpmIdentity::Unknown, TpmIdentity::Hardware));
        assert!(!matches!(TpmIdentity::Software, TpmIdentity::Hardware));
        assert!(matches!(TpmIdentity::Hardware, TpmIdentity::Hardware));
    }

    #[test]
    fn test_tpm2_policy_defaults() {
        let policy = Tpm2Policy::default();
        assert_eq!(policy.pcr_bank, HashingAlgorithm::Sha256);
        assert_eq!(policy.pcrs, vec![7]);
        assert!(policy.auth.is_none());
    }

    #[test]
    fn test_build_pcr_selection_list() {
        let list = build_pcr_selection_list(HashingAlgorithm::Sha256, &[0, 7]);
        assert!(list.is_ok());

        // PCR index >= 32 fails
        let bad = build_pcr_selection_list(HashingAlgorithm::Sha256, &[32]);
        assert!(bad.is_err());
        assert!(bad.unwrap_err().to_string().contains("Invalid PCR index"));
    }
}
