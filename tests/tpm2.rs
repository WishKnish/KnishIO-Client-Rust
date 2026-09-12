#![cfg(feature = "tpm")]

use knishio_client::storage::{
    MemoryStorageBackend, SecretStorageProvider, StorageBackend, StorageOptions,
    Tpm2SecretStorageProvider,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn tpm2_secret_storage_provider_lifecycle() {
    let alias = format!("test_tpm_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = match Tpm2SecretStorageProvider::new(Some(backend.clone()), None, Some(&alias)) {
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
    let provider2 = Tpm2SecretStorageProvider::new(Some(backend.clone()), None, Some(&alias))
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

    let provider_corrupted = Tpm2SecretStorageProvider::new(Some(backend.clone()), None, Some(&alias))
        .expect("create provider with corrupted backend");
    let retrieve_corrupted = provider_corrupted
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await;
    assert!(
        retrieve_corrupted.is_err(),
        "corrupted TPM private key must fail retrieve_secret"
    );
}
