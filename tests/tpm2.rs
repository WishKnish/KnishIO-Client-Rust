#![cfg(feature = "tpm")]

use knishio_client::storage::{
    MemoryStorageBackend, SecretStorageProvider, StorageBackend, StorageOptions,
    Tpm2Policy, Tpm2SecretStorageProvider, TpmIdentity, RECOVERY_KEY_PREFIX,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn tpm2_secret_storage_provider_lifecycle() {
    let alias = format!("test_tpm_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = match Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias), None) {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("TPM unavailable"),
                "unexpected error when TPM absent: {}",
                msg
            );
            println!("TPM unavailable in this environment ({}); skipping positive assertions", msg);
            return;
        }
    };

    assert_eq!(provider.provider_type(), "tpm2-aes-gcm");

    let bundle_hash = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let secret = "MASTER-SECRET-IN-TPM2-ENVELOPE";

    // Round trip
    provider
        .store_secret(bundle_hash, secret, StorageOptions::default())
        .await
        .expect("store secret with TPM2 provider");

    let retrieved = provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve secret")
        .expect("secret exists");
    assert_eq!(retrieved, secret);

    // with_secret works
    let len = provider
        .with_secret(bundle_hash, StorageOptions::default(), |s| Ok(s.len()))
        .await
        .expect("with_secret");
    assert_eq!(len, secret.len());

    // Second instance same alias/backend unseals the first's passphrase
    let provider2 = Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias), None)
        .expect("second instance on existing TPM sealed record");
    let retrieved2 = provider2
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve from second instance")
        .expect("secret exists");
    assert_eq!(retrieved2, secret);

    // Verify hardwareBacked and providerType in emitted metadata
    let raw = backend
        .get_item(&format!("knishio:secret:{}", bundle_hash))
        .unwrap()
        .expect("raw envelope in backend");
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let metadata = json["metadata"].as_object().expect("metadata object");

    assert_eq!(metadata["bundleHash"], serde_json::json!(bundle_hash));
    assert_eq!(
        metadata["hardwareBacked"],
        serde_json::json!(provider.is_hardware_backed())
    );
    assert_eq!(metadata["providerType"], serde_json::json!("tpm2-aes-gcm"));
    assert!(!metadata.contains_key("label"), "label omitted when unset");

    // StorageOptions.passphrase must be rejected
    let bad_options = StorageOptions::with_passphrase("illegal-passphrase");
    let store_bad = provider.store_secret("bundle2", "s2", bad_options.clone()).await;
    assert!(store_bad.is_err());
    assert!(store_bad.unwrap_err().to_string().contains("passphrase is not accepted"));

    // Corrupted private blob returns Err on unseal
    let tpm_record_key = format!("knishio:kek:tpm2:{}", alias);
    let tpm_raw = backend.get_item(&tpm_record_key).unwrap().expect("tpm record");
    let mut tpm_json: serde_json::Value = serde_json::from_str(&tpm_raw).unwrap();
    let original_priv = tpm_json["private"].as_str().unwrap();
    // Flip one char
    let mut corrupted_priv = original_priv.to_string();
    if corrupted_priv.starts_with('A') {
        corrupted_priv.replace_range(..1, "B");
    } else {
        corrupted_priv.replace_range(..1, "A");
    }
    tpm_json["private"] = serde_json::Value::String(corrupted_priv);
    backend.set_item(&tpm_record_key, serde_json::to_string(&tpm_json).unwrap()).unwrap();

    let provider_corrupted = Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias), None)
        .expect("create provider with corrupted backend");
    let retrieve_corrupted = provider_corrupted
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await;
    assert!(
        retrieve_corrupted.is_err(),
        "corrupted TPM private key must fail retrieve_secret"
    );
}

#[test]
fn test_tpm_identity_fail_closed_guarantees() {
    assert!(!matches!(TpmIdentity::Unknown, TpmIdentity::Hardware));
    assert!(!matches!(TpmIdentity::Software, TpmIdentity::Hardware));
    assert!(matches!(TpmIdentity::Hardware, TpmIdentity::Hardware));
}

#[test]
fn test_tpm2_policy_struct_and_builder() {
    use tss_esapi::interface_types::algorithm::HashingAlgorithm;

    let default_policy = Tpm2Policy::default();
    assert_eq!(default_policy.pcr_bank, HashingAlgorithm::Sha256);
    assert_eq!(default_policy.pcrs, vec![7]);
    assert!(default_policy.auth.is_none());

    let custom_policy = Tpm2Policy::with_pcrs(HashingAlgorithm::Sha256, vec![0, 1, 7]);
    assert_eq!(custom_policy.pcrs, vec![0, 1, 7]);
}

#[tokio::test]
async fn tpm2_secret_storage_provider_recovery_lifecycle() {
    let alias = format!("test_tpm_rec_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = match Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias), None) {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            println!("TPM unavailable ({}); skipping recovery test", msg);
            return;
        }
    };

    let bundle_hash = "1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff";
    let secret = "SECRET-WITH-TPM2-RECOVERY";
    let recovery_pass = "tpm2-recovery-passphrase-42";

    let store_opts = StorageOptions::default().with_recovery_passphrase(recovery_pass);
    provider
        .store_secret(bundle_hash, secret, store_opts)
        .await
        .expect("store secret with recovery");

    // Both primary and recovery records exist in backend
    assert!(backend.get_item(&format!("knishio:secret:{}", bundle_hash)).unwrap().is_some());
    let rec_key = format!("{}{}", RECOVERY_KEY_PREFIX, bundle_hash);
    assert!(backend.get_item(&rec_key).unwrap().is_some());

    // list_secrets ignores recovery record
    let secrets = provider.list_secrets().await.expect("list secrets");
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].bundle_hash, bundle_hash);

    // Corrupt TPM primary record to simulate KEK loss / TPM reset
    let tpm_record_key = format!("knishio:kek:tpm2:{}", alias);
    backend.remove_item(&tpm_record_key).unwrap();

    // Create a new provider on the same backend (missing KEK)
    let alias2 = format!("test_tpm_rec2_{}", Uuid::new_v4().simple());
    let provider2 = Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias2), None)
        .expect("create new provider");

    // Primary retrieval fails (old envelope cannot be decrypted by provider2 with new KEK)
    let retrieve_fail = provider2.retrieve_secret(bundle_hash, StorageOptions::default()).await;
    assert!(retrieve_fail.is_err(), "retrieval under different TPM KEK must fail");

    // Recovery restores the secret under the new provider's KEK
    provider2
        .recover_secret(bundle_hash, recovery_pass, StorageOptions::default())
        .await
        .expect("recover secret with recovery passphrase");

    let recovered = provider2
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve recovered secret")
        .expect("recovered secret exists");
    assert_eq!(recovered, secret);

    // Deleting removes both primary and recovery records
    provider2.delete_secret(bundle_hash).await.expect("delete secret");
    assert!(backend.get_item(&format!("knishio:secret:{}", bundle_hash)).unwrap().is_none());
    assert!(backend.get_item(&rec_key).unwrap().is_none());
}

#[tokio::test]
async fn tpm2_policy_fail_closed_without_recovery() {
    use tss_esapi::interface_types::algorithm::HashingAlgorithm;

    let alias = format!("test_tpm_pol_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());
    let policy = Tpm2Policy::with_pcrs(HashingAlgorithm::Sha256, vec![7]);

    let provider = match Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias), Some(policy)) {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            println!("TPM unavailable ({}); skipping policy fail-closed test", msg);
            return;
        }
    };

    let bundle_hash = "beefbeefbeefbeefbeefbeefbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef";
    let secret = "SECRET-UNDER-PCR-POLICY";

    // 1. Storing without recovery or allow_unrecoverable must fail closed
    let err = provider
        .store_secret(bundle_hash, secret, StorageOptions::default())
        .await;
    assert!(err.is_err(), "storing with PCR policy without recovery must fail");
    let err_msg = err.unwrap_err().to_string();
    assert!(
        err_msg.contains("requires recovery_passphrase"),
        "unexpected error: {err_msg}"
    );

    // 2. Storing with allow_unrecoverable succeeds
    let opts_unrec = StorageOptions::default().with_allow_unrecoverable(true);
    provider
        .store_secret(bundle_hash, secret, opts_unrec)
        .await
        .expect("store with allow_unrecoverable");

    let retrieved = provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve secret")
        .expect("secret exists");
    assert_eq!(retrieved, secret);

    // 3. Storing with recovery passphrase succeeds and records policy info
    let bundle_hash2 = "cafebeefcafebeefcafebeefcafebeefcafebeefcafebeefcafebeefcafebeef";
    let opts_rec = StorageOptions::default().with_recovery_passphrase("pcr-recovery-pass");
    provider
        .store_secret(bundle_hash2, secret, opts_rec)
        .await
        .expect("store with recovery passphrase");

    let tpm_record_key = format!("knishio:kek:tpm2:{}", alias);
    let tpm_raw = backend.get_item(&tpm_record_key).unwrap().expect("tpm record");
    let tpm_json: serde_json::Value = serde_json::from_str(&tpm_raw).unwrap();
    assert!(tpm_json.get("policy").is_some(), "sealed record must contain policy info");
    assert_eq!(tpm_json["policy"]["bank"], "sha256");
}

#[tokio::test]
async fn tpm2_policy_pcr_extend_fails_unseal() {
    use std::str::FromStr;
    use tss_esapi::{
        handles::PcrHandle,
        interface_types::algorithm::HashingAlgorithm,
        structures::{Digest, DigestValues},
        tcti_ldr::TctiNameConf,
        Context,
    };

    let alias = format!("test_tpm_pcr_ext_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());
    let policy = Tpm2Policy::with_pcrs(HashingAlgorithm::Sha256, vec![7]);

    let provider = match Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias), Some(policy)) {
        Ok(p) => p,
        Err(e) => {
            println!("TPM unavailable ({}); skipping pcr_extend negative test", e);
            return;
        }
    };

    let bundle_hash = "1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff";
    let secret = "SECRET-SEALED-AGAINST-PCR7";

    // 1. Store secret under PCR 7 policy with allow_unrecoverable
    let opts = StorageOptions::default().with_allow_unrecoverable(true);
    provider
        .store_secret(bundle_hash, secret, opts)
        .await
        .expect("store secret under PCR 7 policy");

    // 2. Initial retrieval succeeds while PCR 7 measurement is unaltered
    let retrieved = provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve before pcr_extend")
        .expect("secret exists");
    assert_eq!(retrieved, secret);

    // 3. Mutate PCR 7 measurement via direct TPM context
    let tcti_str = std::env::var("KNISHIO_TPM_TCTI").unwrap_or_else(|_| "device:/dev/tpmrm0".to_string());
    let tcti_conf = match TctiNameConf::from_str(&tcti_str) {
        Ok(c) => c,
        Err(e) => {
            println!("TCTI parse failed ({}); skipping pcr_extend", e);
            return;
        }
    };
    let mut direct_context = match Context::new(tcti_conf) {
        Ok(c) => c,
        Err(e) => {
            println!("Direct TPM context creation failed ({}); skipping pcr_extend", e);
            return;
        }
    };

    let mut dv = DigestValues::new();
    let d = Digest::try_from(vec![0x42u8; 32]).expect("valid 32-byte digest");
    dv.set(HashingAlgorithm::Sha256, d);
    direct_context.set_sessions((Some(tss_esapi::interface_types::session_handles::AuthSession::Password), None, None));
    direct_context
        .pcr_extend(PcrHandle::Pcr7, dv)
        .expect("extend PCR 7 to mutate state");

    // 4. Create a fresh provider instance on the same backend record.
    // This forces unsealing the KeyedHash under the drifted PCR 7 state.
    let provider_after_drift = Tpm2SecretStorageProvider::new(backend.clone(), None, Some(&alias), None)
        .expect("instantiate provider after PCR drift");

    let err = provider_after_drift
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await;

    // 5. Unseal must fail closed with an error (not panic)
    assert!(
        err.is_err(),
        "unsealing after PCR 7 drift must fail authorization"
    );
    let err_str = err.unwrap_err().to_string();
    assert!(
        err_str.contains("TPM unseal failed") || err_str.contains("policy_pcr failed") || err_str.contains("DecryptionFailed"),
        "expected policy/unseal failure error, got: {err_str}"
    );
}
