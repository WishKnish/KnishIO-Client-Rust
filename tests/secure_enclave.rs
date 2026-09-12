#![cfg(all(feature = "secure-enclave", target_os = "macos"))]

use knishio_client::storage::{
    AesGcmSecretStorageProvider, MemoryStorageBackend, SecretStorageProvider, StorageBackend,
    StorageOptions, SecureEnclaveSecretStorageProvider, RECOVERY_KEY_PREFIX,
};
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

macro_rules! init_provider_or_skip {
    ($backend:expr, $alias:expr) => {
        match SecureEnclaveSecretStorageProvider::new($backend, Some(&$alias)) {
            Ok(p) => p,
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("Secure Enclave unavailable"),
                    "unexpected error when Secure Enclave absent or restricted: {}",
                    msg
                );
                println!(
                    "Secure Enclave unavailable in this environment ({}); skipping positive assertions",
                    msg
                );
                return;
            }
        }
    };
}

/// Positive-path Secure Enclave custody test. Requires codesigned binary with keychain-access-groups
/// entitlement (e.g. within an macOS .app bundle or signed with an Apple Developer identity).
///
/// To run on an entitled Apple Silicon binary:
/// `codesign -f -s - --entitlements se.entitlements <test_binary>`
/// `cargo test --features secure-enclave --test secure_enclave -- --ignored`
#[tokio::test]
#[ignore = "requires codesigned binary with keychain-access-groups entitlement"]
async fn secure_enclave_custody_lifecycle_and_key_loss_recovery() {
    let alias = format!("test_se_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = init_provider_or_skip!(backend.clone(), alias);

    assert_eq!(provider.provider_type(), "secure-enclave-aes-gcm");
    assert!(provider.is_hardware_backed());
    assert!(provider.is_available().await);

    let bundle_hash = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let secret = "MASTER-SECRET-IN-SECURE-ENCLAVE";

    let options = StorageOptions {
        label: Some("SE Test Key".to_string()),
        passphrase: None,
        recovery_passphrase: Some(Zeroizing::new("rec-pass-12345".to_string())),
        allow_unrecoverable: false,
    };

    // Store secret
    provider
        .store_secret(bundle_hash, secret, options)
        .await
        .expect("store secret with SE provider");

    // Has secret
    assert!(provider.has_secret(bundle_hash).await.expect("has_secret"));

    // List secrets metadata
    let list = provider.list_secrets().await.expect("list_secrets");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].bundle_hash, bundle_hash);
    assert_eq!(list[0].provider_type, "secure-enclave-aes-gcm");
    assert!(list[0].hardware_backed);
    assert_eq!(list[0].label.as_deref(), Some("SE Test Key"));

    // Retrieve secret
    let retrieved = provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve_secret")
        .expect("secret exists");
    assert_eq!(retrieved, secret);

    // Verify recovery envelope exists in backend
    let rec_raw = backend
        .get_item(&format!("{RECOVERY_KEY_PREFIX}{bundle_hash}"))
        .expect("get recovery envelope")
        .expect("recovery envelope exists");
    assert!(rec_raw.contains("\"providerType\":\"aes-gcm\""));
    assert!(rec_raw.contains("\"hardwareBacked\":false"));

    // Cross-provider verification: recover via AesGcmSecretStorageProvider directly
    let aes_provider = AesGcmSecretStorageProvider::new(Some(backend.clone()), Some("rec-pass-12345".to_string()));
    let recovered_via_aes = aes_provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve via AesGcm recovery record");
    assert_eq!(recovered_via_aes.as_deref(), Some(secret));

    // Simulate key loss: unenroll provider (deletes hardware key and KEK record)
    provider.unenroll().expect("unenroll");

    // Now construct a fresh provider with the same alias (generates fresh hardware key & KEK)
    let fresh_provider = init_provider_or_skip!(backend.clone(), alias);

    // The old primary envelope cannot be unwrapped by the fresh provider
    let unwrap_result = fresh_provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await;
    assert!(
        unwrap_result.is_err(),
        "retrieval of old envelope under new SE key must fail with decryption error"
    );

    // Recover secret using recovery passphrase
    fresh_provider
        .recover_secret(bundle_hash, "rec-pass-12345", StorageOptions::default())
        .await
        .expect("recover_secret into fresh provider");

    // Now retrieval succeeds again under the re-enrolled active key
    let restored = fresh_provider
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve after recovery")
        .expect("secret exists");
    assert_eq!(restored, secret);

    // Delete secret
    assert!(fresh_provider.delete_secret(bundle_hash).await.expect("delete"));
    assert!(!fresh_provider.has_secret(bundle_hash).await.expect("has_secret after delete"));

    // Clean up
    fresh_provider.unenroll().expect("unenroll clean up");
}

#[tokio::test]
async fn secure_enclave_requires_recovery_passphrase_unless_allowed() {
    let alias = format!("test_se_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = init_provider_or_skip!(backend.clone(), alias);

    let bundle_hash = "1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff";
    let secret = "TEST-RECOVERY-ENFORCEMENT";

    // Attempt store without recovery_passphrase and allow_unrecoverable = false -> MUST fail
    let opts_disallowed = StorageOptions {
        label: None,
        passphrase: None,
        recovery_passphrase: None,
        allow_unrecoverable: false,
    };
    let err = provider
        .store_secret(bundle_hash, secret, opts_disallowed)
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("requires recovery_passphrase unless allow_unrecoverable is true"),
        "expected recovery_passphrase requirement error, got: {msg}"
    );

    // Store with allow_unrecoverable = true -> succeeds
    let opts_allowed = StorageOptions {
        label: None,
        passphrase: None,
        recovery_passphrase: None,
        allow_unrecoverable: true,
    };
    provider
        .store_secret(bundle_hash, secret, opts_allowed)
        .await
        .expect("store with allow_unrecoverable succeeded");

    provider.unenroll().expect("unenroll clean up");
}

#[tokio::test]
async fn secure_enclave_rejects_caller_passphrase() {
    let alias = format!("test_se_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider = init_provider_or_skip!(backend.clone(), alias);

    let bundle_hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let secret = "TEST-PASSPHRASE-REJECTION";

    let opts_with_pass = StorageOptions {
        label: None,
        passphrase: Some("caller-supplied-passphrase".to_string()),
        recovery_passphrase: None,
        allow_unrecoverable: true,
    };

    let err = provider
        .store_secret(bundle_hash, secret, opts_with_pass)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("derives its passphrase from the Secure Enclave"),
        "expected passphrase rejection, got: {err}"
    );

    provider.unenroll().expect("unenroll clean up");
}

/// Positive-path Secure Enclave multi-instance persistence test.
/// Requires codesigned binary with keychain-access-groups entitlement.
#[tokio::test]
#[ignore = "requires codesigned binary with keychain-access-groups entitlement"]
async fn secure_enclave_persists_across_instances() {
    let alias = format!("test_se_{}", Uuid::new_v4().simple());
    let backend = Arc::new(MemoryStorageBackend::new());

    let provider1 = init_provider_or_skip!(backend.clone(), alias);

    let bundle_hash = "9999888877776666555544443333222211110000fedcba9876543210fedcba98";
    let secret = "PERSISTENT-SECRET-ACROSS-INSTANCES";

    let opts = StorageOptions {
        label: Some("Persistent Key".to_string()),
        passphrase: None,
        recovery_passphrase: None,
        allow_unrecoverable: true,
    };

    provider1
        .store_secret(bundle_hash, secret, opts)
        .await
        .expect("store with provider1");

    // Drop provider 1
    drop(provider1);

    // Create provider 2 with same backend and same alias
    let provider2 = init_provider_or_skip!(backend.clone(), alias);

    let retrieved = provider2
        .retrieve_secret(bundle_hash, StorageOptions::default())
        .await
        .expect("retrieve with provider2")
        .expect("secret exists");
    assert_eq!(retrieved, secret);

    provider2.unenroll().expect("unenroll clean up");
}
