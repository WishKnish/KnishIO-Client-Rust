#![cfg(feature = "keyring")]

use knishio_client::storage::{
    MemoryStorageBackend, OsKeychainSecretStorageProvider, SecretStorageProvider, StorageBackend,
    StorageOptions,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn os_keychain_secret_storage_provider_lifecycle() {
    let service = format!("io.knish.test.{}", Uuid::new_v4());
    let alias = "test_wallet";
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = match OsKeychainSecretStorageProvider::new(
        Some(backend.clone()),
        Some(&service),
        Some(alias),
    ) {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("OS keychain unavailable"),
                "unexpected error from OS keychain: {}",
                msg
            );
            println!("OS keychain unavailable in this environment ({}); skipping positive assertions", msg);
            return;
        }
    };

    let user = format!("knishio:kek:{}", alias);
    let entry = keyring::Entry::new(&service, &user).expect("keyring entry");

    assert_eq!(provider.provider_type(), "os-keychain-aes-gcm");
    assert!(!provider.is_hardware_backed());
    // Round trip
    let bundle_hash = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let secret = "SECRET-IN-OS-KEYCHAIN";

    provider
        .store_secret(bundle_hash, secret, StorageOptions::default())
        .await
        .expect("store secret in os keychain provider");

    let retrieved = provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve secret")
        .expect("secret exists");
    assert_eq!(retrieved, secret);

    // Second instance same service/alias/backend reads the secret
    let provider2 = OsKeychainSecretStorageProvider::new(
        Some(backend.clone()),
        Some(&service),
        Some(alias),
    )
    .expect("second instance on existing keychain");
    let retrieved2 = provider2
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve from second instance")
        .expect("secret exists");
    assert_eq!(retrieved2, secret);

    // StorageOptions.passphrase must be rejected
    let bad_options = StorageOptions::with_passphrase("illegal-passphrase");
    let store_bad = provider.store_secret("bundle2", "secret2", bad_options.clone()).await;
    assert!(store_bad.is_err());
    assert!(store_bad.unwrap_err().to_string().contains("passphrase is not accepted"));

    let retrieve_bad = provider.retrieve_secret(bundle_hash, bad_options).await;
    assert!(retrieve_bad.is_err());
    assert!(retrieve_bad.unwrap_err().to_string().contains("passphrase is not accepted"));

    // Check metadata contract in backend
    let raw = backend
        .get_item(&format!("knishio:secret:{}", bundle_hash))
        .unwrap()
        .expect("raw envelope in backend");
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let metadata = json["metadata"].as_object().expect("metadata object");

    assert_eq!(metadata["bundleHash"], serde_json::json!(bundle_hash));
    assert_eq!(metadata["hardwareBacked"], serde_json::json!(false));
    assert_eq!(metadata["providerType"], serde_json::json!("os-keychain-aes-gcm"));
    assert!(!metadata.contains_key("label"), "label omitted when unset");

    // Cleanup credential from OS keychain
    let _ = entry.delete_credential();
}
