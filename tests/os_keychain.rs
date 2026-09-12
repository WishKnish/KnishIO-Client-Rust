#![cfg(feature = "keyring")]

use knishio_client::storage::{
    MemoryStorageBackend, OsKeychainSecretStorageProvider, SecretStorageProvider, StorageBackend,
    StorageOptions, RECOVERY_KEY_PREFIX,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn os_keychain_secret_storage_provider_lifecycle() {
    let service = format!("io.knish.test.{}", Uuid::new_v4());
    let alias = "test_wallet";
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = match OsKeychainSecretStorageProvider::new(
        backend.clone(),
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
        backend.clone(),
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

#[tokio::test]
async fn os_keychain_secret_storage_provider_recovery_lifecycle() {
    let service = format!("io.knish.test.{}", Uuid::new_v4());
    let alias1 = "test_wallet_orig";
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider1 = match OsKeychainSecretStorageProvider::new(
        backend.clone(),
        Some(&service),
        Some(alias1),
    ) {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            println!("OS keychain unavailable ({}); skipping recovery test", msg);
            return;
        }
    };

    let bundle_hash = "deadbeef1111222233334444555566667777888899990000aaaabbbbccccdddd";
    let secret = "SECRET-FOR-OS-KEYCHAIN-RECOVERY";
    let recovery_pass = "keychain-recovery-passphrase-99";

    let store_opts = StorageOptions::default().with_recovery_passphrase(recovery_pass);
    provider1
        .store_secret(bundle_hash, secret, store_opts)
        .await
        .expect("store secret with recovery");

    // Both primary and recovery records exist in backend
    let prim_key = format!("knishio:secret:{}", bundle_hash);
    let rec_key = format!("{}{}", RECOVERY_KEY_PREFIX, bundle_hash);
    assert!(backend.get_item(&prim_key).unwrap().is_some());
    assert!(backend.get_item(&rec_key).unwrap().is_some());

    // list_secrets ignores recovery record
    let secrets = provider1.list_secrets().await.expect("list secrets");
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].bundle_hash, bundle_hash);

    // Delete the original keychain credential to simulate KEK loss
    let user1 = format!("knishio:kek:{}", alias1);
    let entry1 = keyring::Entry::new(&service, &user1).expect("keyring entry");
    let _ = entry1.delete_credential();

    // Create a new provider on a new alias (fresh KEK)
    let alias2 = "test_wallet_new";
    let provider2 = OsKeychainSecretStorageProvider::new(
        backend.clone(),
        Some(&service),
        Some(alias2),
    )
    .expect("create provider with new alias");

    // Primary retrieval fails because provider2 doesn't have the original KEK
    let retrieve_fail = provider2.retrieve_secret(bundle_hash, StorageOptions::default()).await;
    assert!(retrieve_fail.is_err(), "retrieval under different keychain KEK must fail");

    // Recovery re-enrolls the secret under provider2's KEK
    provider2
        .recover_secret(bundle_hash, recovery_pass, StorageOptions::default())
        .await
        .expect("recover secret");

    let recovered = provider2
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve recovered secret")
        .expect("secret exists");
    assert_eq!(recovered, secret);

    // delete_secret removes both primary and recovery records
    provider2.delete_secret(bundle_hash).await.expect("delete secret");
    assert!(backend.get_item(&prim_key).unwrap().is_none());
    assert!(backend.get_item(&rec_key).unwrap().is_none());

    // Cleanup
    let user2 = format!("knishio:kek:{}", alias2);
    let entry2 = keyring::Entry::new(&service, &user2).expect("keyring entry");
    let _ = entry2.delete_credential();
}
