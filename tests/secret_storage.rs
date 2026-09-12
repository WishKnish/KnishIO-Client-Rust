use knishio_client::client::KnishIOClient;
use knishio_client::crypto::{generate_bundle_hash, generate_secret};
use knishio_client::error::KnishIOError;
use knishio_client::storage::secure_memory::{constant_time_equals, with_secure_bytes, zeroize_bytes};
use knishio_client::storage::{
    AesGcmSecretStorageProvider, EncryptedSecretPayload, MemorySecretStorageProvider,
    MemoryStorageBackend, SecretStorageProvider, StorageBackend, StorageOptions,
    RECOVERY_KEY_PREFIX,
};
use knishio_client::wallet::Wallet;
use knishio_client::atom::Atom;
use std::sync::Arc;

#[test]
fn test_secure_memory_zeroize_and_constant_time() {
    let mut bytes = vec![1u8, 2, 3, 4, 5];
    zeroize_bytes(&mut bytes);
    assert_eq!(bytes, vec![0u8, 0, 0, 0, 0]);

    let data = vec![42u8, 43, 44];
    let observed = with_secure_bytes(data, |b| b[0]);
    assert_eq!(observed, 42);

    assert!(constant_time_equals(b"secret123", b"secret123"));
    assert!(!constant_time_equals(b"secret123", b"secret124"));
    assert!(!constant_time_equals(b"secret123", b"secret12"));
}

#[tokio::test]
async fn test_memory_secret_storage_provider() {
    let provider = MemorySecretStorageProvider::new();

    assert_eq!(provider.provider_type(), "memory");
    assert!(!provider.is_hardware_backed());
    assert!(provider.is_available().await);

    let bundle = "bundle_hash_test_123";
    let secret = "master_secret_value_xyz";

    assert!(!provider.has_secret(bundle).await.unwrap());
    assert!(provider.retrieve_secret(bundle, StorageOptions::default()).await.unwrap().is_none());

    let opts = StorageOptions::new(Some("Test Key".to_string()), None);
    provider.store_secret(bundle, secret, opts).await.unwrap();

    assert!(provider.has_secret(bundle).await.unwrap());
    assert_eq!(
        provider.retrieve_secret(bundle, StorageOptions::default()).await.unwrap(),
        Some(secret.to_string())
    );

    let list = provider.list_secrets().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].bundle_hash, bundle);
    assert_eq!(list[0].label.as_deref(), Some("Test Key"));

    let len = provider.with_secret(bundle, StorageOptions::default(), |s| Ok(s.len())).await.unwrap();
    assert_eq!(len, secret.len());

    assert!(provider.delete_secret(bundle).await.unwrap());
    assert!(!provider.has_secret(bundle).await.unwrap());
}

#[tokio::test]
async fn test_aes_gcm_secret_storage_provider() {
    let provider = AesGcmSecretStorageProvider::new(
        None,
        Some("client-secure-passphrase".to_string()),
    );

    assert!(provider.is_available().await);
    assert_eq!(provider.provider_type(), "aes-gcm");
    assert!(!provider.is_hardware_backed());

    let bundle = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let secret = "a".repeat(256);

    let opts = StorageOptions::new(Some("Prod Seed".to_string()), None);
    provider.store_secret(bundle, &secret, opts).await.unwrap();

    assert!(provider.has_secret(bundle).await.unwrap());

    let retrieved = provider.retrieve_secret(bundle, StorageOptions::default()).await.unwrap();
    assert_eq!(retrieved, Some(secret.clone()));

    let prefix = provider.with_secret(bundle, StorageOptions::default(), |s| Ok(s[..10].to_string())).await.unwrap();
    assert_eq!(prefix, "aaaaaaaaaa");

    let list = provider.list_secrets().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].bundle_hash, bundle);
    assert_eq!(list[0].label.as_deref(), Some("Prod Seed"));

    // Wrong passphrase fails
    let wrong_opts = StorageOptions::with_passphrase("wrong-password");
    let err = provider.retrieve_secret(bundle, wrong_opts.clone()).await;
    assert!(err.is_err());
    match err.unwrap_err() {
        KnishIOError::DecryptionFailed(_) => (),
        other => panic!("Expected DecryptionFailed, got: {:?}", other),
    }

    assert!(provider.delete_secret(bundle).await.unwrap());
    assert!(!provider.has_secret(bundle).await.unwrap());
}

#[tokio::test]
async fn test_knishio_client_secret_storage_integration() {
    let test_seed = "knishio-rust-hardware-storage-test";
    let canonical_secret = generate_secret(test_seed);
    let canonical_bundle = generate_bundle_hash(&canonical_secret);

    let storage = Arc::new(AesGcmSecretStorageProvider::new(
        None,
        Some("client-secure-pass".to_string()),
    ));

    storage.store_secret(&canonical_bundle, &canonical_secret, StorageOptions::default()).await.unwrap();

    let mut client = KnishIOClient::new(
        "https://api.test.knish.io/graphql",
        None,
        None,
        None,
        None,
        None,
    );
    client.set_secret_storage(storage.clone(), Some(canonical_bundle.clone()));

    assert!(client.has_secret());
    assert!(client.has_bundle());
    assert_eq!(client.get_bundle(), Some(canonical_bundle.as_str()));

    // Internal secret property is None (no cleartext retention in client heap)
    assert!(client.get_secret().is_err());

    // Can retrieve secret via storage
    let retrieved = client.retrieve_secret(StorageOptions::default()).await.unwrap();
    assert_eq!(retrieved, Some(canonical_secret.clone()));

    // Create a source wallet to provide to create_molecule
    let source_wallet = Wallet::create(
        Some(&canonical_secret),
        Some(&canonical_bundle),
        "USER",
        None,
        Some(&"0".repeat(64)),
        None,
    ).unwrap();

    // create_molecule unwraps from storage, sets up remainder wallet, and sets molecule.bundle
    let mut molecule = client.create_molecule(
        None,
        None,
        Some(source_wallet.clone()),
        None,
    ).await.unwrap();

    assert_eq!(molecule.bundle.as_deref(), Some(canonical_bundle.as_str()));
    assert!(molecule.source_wallet.is_some());
    assert!(molecule.remainder_wallet.is_some());

    // Sign the molecule
    let atom = Atom::new(
        "0".repeat(64),
        source_wallet.address.as_deref().unwrap_or(""),
        knishio_client::types::Isotope::C,
        "USER",
    );
    molecule.add_atom(atom);
    let signature = molecule.sign(None, false, true).unwrap();
    assert!(signature.is_some());
    assert!(molecule.molecular_hash.is_some());
}

#[tokio::test]
async fn test_knishio_client_set_secret_auto_sync_and_reset() {
    let test_seed = "knishio-rust-auto-sync-test";
    let canonical_secret = generate_secret(test_seed);
    let canonical_bundle = generate_bundle_hash(&canonical_secret);

    let mut client = KnishIOClient::new(
        "https://api.test.knish.io/graphql",
        None,
        None,
        None,
        None,
        None,
    );

    assert!(!client.has_secret());
    assert!(client.get_secret_storage().is_none());

    client.set_secret(&canonical_secret);

    assert!(client.has_secret());
    assert_eq!(client.get_bundle(), Some(canonical_bundle.as_str()));
    assert_eq!(client.get_secret().unwrap(), canonical_secret.as_str());

    let storage = client.get_secret_storage();
    assert!(storage.is_some());
    let storage = storage.unwrap();
    assert!(storage.has_secret(&canonical_bundle).await.unwrap());
    assert_eq!(
        storage.retrieve_secret(&canonical_bundle, StorageOptions::default()).await.unwrap(),
        Some(canonical_secret.clone())
    );

    client.reset();

    assert!(!client.has_secret());
    assert!(client.get_secret_storage().is_none());
    assert!(!client.has_bundle());
}

// ---------------------------------------------------------------------------
// Cross-SDK envelope interop
//
// The AES-GCM envelope is a shared wire format: TS, JS, and Kotlin all emit
// camelCase metadata keys. Rust 0.9.5 shipped snake_case, so a TS-produced
// envelope failed here with `missing field bundle_hash` BEFORE decryption was
// attempted, and a Rust-produced envelope deserialized in TS with every typed
// metadata field `undefined`. The crypto core was always identical; only the
// framing diverged, which is exactly the class of bug that reading the code
// side-by-side cannot catch.
//
// FROZEN_TS_ENVELOPE below was produced by @wishknish/knishio-client-ts@0.9.7's
// WebCryptoSecretStorageProvider (PBKDF2-HMAC-SHA256 x100000, 16-byte salt,
// 12-byte IV, AES-256-GCM, standard base64). Do not regenerate it to make a
// failure go away - it is the contract.
// ---------------------------------------------------------------------------

#[path = "fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    FROZEN_LEGACY_0_9_5_ENVELOPE, FROZEN_TS_ENVELOPE, XSDK_BUNDLE, XSDK_PASSPHRASE, XSDK_PLAINTEXT,
};
async fn decrypt_frozen(envelope: &str) -> Result<Option<String>, KnishIOError> {
    let backend = Arc::new(MemoryStorageBackend::new());
    backend
        .set_item(
            &format!("knishio:secret:{}", XSDK_BUNDLE),
            envelope.to_string(),
        )
        .unwrap();
    AesGcmSecretStorageProvider::new(Some(backend), None)
        .retrieve_secret(XSDK_BUNDLE, StorageOptions::with_passphrase(XSDK_PASSPHRASE))
        .await
}

#[tokio::test]
async fn decrypts_envelope_written_by_the_typescript_sdk() {
    assert_eq!(
        decrypt_frozen(FROZEN_TS_ENVELOPE).await.unwrap(),
        Some(XSDK_PLAINTEXT.to_string()),
        "a TS-produced envelope must decrypt here; snake_case metadata keys break this"
    );
}

#[tokio::test]
async fn decrypts_legacy_snake_case_envelope_written_by_0_9_5() {
    assert_eq!(
        decrypt_frozen(FROZEN_LEGACY_0_9_5_ENVELOPE).await.unwrap(),
        Some(XSDK_PLAINTEXT.to_string()),
        "secrets stored by 0.9.5 must survive the camelCase migration"
    );
}

#[tokio::test]
async fn emits_camel_case_metadata_for_peer_sdks() {
    let backend = Arc::new(MemoryStorageBackend::new());
    let provider = AesGcmSecretStorageProvider::new(Some(backend.clone()), None);
    provider
        .store_secret(
            XSDK_BUNDLE,
            XSDK_PLAINTEXT,
            StorageOptions::with_passphrase(XSDK_PASSPHRASE),
        )
        .await
        .unwrap();

    let raw = backend
        .get_item(&format!("knishio:secret:{}", XSDK_BUNDLE))
        .unwrap()
        .expect("envelope stored");
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let metadata = json["metadata"].as_object().expect("metadata object");

    for key in ["bundleHash", "createdAt", "hardwareBacked", "providerType"] {
        assert!(
            metadata.contains_key(key),
            "peer SDKs read `{}`; emitted keys were {:?}",
            key,
            metadata.keys().collect::<Vec<_>>()
        );
    }
    for key in ["bundle_hash", "created_at", "hardware_backed", "provider_type"] {
        assert!(
            !metadata.contains_key(key),
            "`{}` is the 0.9.5 divergence and must not be emitted",
            key
        );
    }
    assert!(
        !metadata.contains_key("label"),
        "optional metadata keys like `label` must be omitted when unset, never emitted as null"
    );

    // When label is provided, it must be emitted
    provider
        .store_secret(
            XSDK_BUNDLE,
            XSDK_PLAINTEXT,
            StorageOptions::new(Some("probe".into()), Some(XSDK_PASSPHRASE.into())),
        )
        .await
        .unwrap();
    let raw_with_label = backend
        .get_item(&format!("knishio:secret:{}", XSDK_BUNDLE))
        .unwrap()
        .expect("envelope stored");
    let json_with_label: serde_json::Value = serde_json::from_str(&raw_with_label).unwrap();
    assert_eq!(json_with_label["metadata"]["label"], serde_json::json!("probe"));
    assert_eq!(metadata["hardwareBacked"], serde_json::json!(false), "a software provider must never emit hardwareBacked=true");
    assert_eq!(metadata["providerType"], serde_json::json!("aes-gcm"));
}

#[tokio::test]
async fn file_storage_backend_round_trip_permissions_and_corruption() {
    use knishio_client::storage::FileStorageBackend;
    use std::fs;

    let temp_path = std::env::temp_dir().join(format!("knishio-fsb-{}.json", uuid::Uuid::new_v4()));

    // Scope 1: write items and check permissions
    {
        let backend1 = FileStorageBackend::new(&temp_path).expect("create file storage");
        assert_eq!(backend1.get_item("key1").unwrap(), None);
        backend1.set_item("key1", "val1".into()).unwrap();
        assert_eq!(backend1.get_item("key1").unwrap(), Some("val1".into()));
        assert!(backend1.keys().unwrap().contains(&"key1".to_string()));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = fs::metadata(&temp_path).expect("read metadata");
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "file mode must be 0600 on unix, got {:o}", mode);
        }
    }

    // Scope 2: second instance reads existing file
    {
        let backend2 = FileStorageBackend::new(&temp_path).expect("open existing file storage");
        assert_eq!(backend2.get_item("key1").unwrap(), Some("val1".into()));
        assert!(backend2.remove_item("key1").unwrap());
        assert_eq!(backend2.get_item("key1").unwrap(), None);
        assert!(!backend2.remove_item("key1").unwrap());
    }

    // Corrupt the file
    fs::write(&temp_path, "not valid json").unwrap();
    let corrupt_res = FileStorageBackend::new(&temp_path);
    assert!(corrupt_res.is_err(), "corrupted store must return Err");

    // Cleanup
    let _ = fs::remove_file(&temp_path);
}

#[tokio::test]
async fn test_aes_gcm_secret_storage_recovery_lifecycle() {
    let backend = Arc::new(MemoryStorageBackend::new());
    let provider = AesGcmSecretStorageProvider::new(Some(backend.clone()), Some("primary-pass".into()));

    let bundle = "bundle_rec_aes_gcm_1234567890abcdef";
    let secret = "MASTER-SECRET-AES-GCM-RECOVERY-TEST";
    let recovery_pass = "recovery-passphrase-secret-77";

    let opts = StorageOptions::default().with_recovery_passphrase(recovery_pass);
    provider.store_secret(bundle, secret, opts).await.unwrap();

    // Both primary and recovery records exist in backend
    let prim_key = format!("knishio:secret:{}", bundle);
    let rec_key = format!("{}{}", RECOVERY_KEY_PREFIX, bundle);
    assert!(backend.get_item(&prim_key).unwrap().is_some());
    let rec_raw = backend.get_item(&rec_key).unwrap().expect("recovery record");

    // Recovery envelope metadata matches contract
    let rec_payload: EncryptedSecretPayload = serde_json::from_str(&rec_raw).unwrap();
    assert_eq!(rec_payload.metadata.bundle_hash, bundle);
    assert_eq!(rec_payload.metadata.provider_type, "aes-gcm");
    assert!(!rec_payload.metadata.hardware_backed);

    // list_secrets ignores recovery record
    let list = provider.list_secrets().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].bundle_hash, bundle);

    // Corrupt primary record to simulate key loss
    backend.remove_item(&prim_key).unwrap();
    assert_eq!(provider.retrieve_secret(bundle, StorageOptions::default()).await.unwrap(), None);

    // Wrong recovery passphrase fails with DecryptionFailed
    let wrong_res = provider.recover_secret(bundle, "wrong-recovery-pass", StorageOptions::default()).await;
    assert!(wrong_res.is_err());
    assert!(matches!(wrong_res.unwrap_err(), KnishIOError::DecryptionFailed(_)));

    // Recover secret re-enrolls under new primary passphrase
    let new_opts = StorageOptions::with_passphrase("brand-new-primary-pass");
    provider.recover_secret(bundle, recovery_pass, new_opts.clone()).await.unwrap();

    // Primary retrieval with new passphrase succeeds
    let recovered = provider.retrieve_secret(bundle, new_opts).await.unwrap().expect("recovered secret");
    assert_eq!(recovered, secret);

    // delete_secret removes both primary and recovery keys
    assert!(provider.delete_secret(bundle).await.unwrap());
    assert!(backend.get_item(&prim_key).unwrap().is_none());
    assert!(backend.get_item(&rec_key).unwrap().is_none());
}

#[tokio::test]
async fn test_memory_secret_storage_recovery_lifecycle() {
    let provider = MemorySecretStorageProvider::new();

    let bundle = "bundle_rec_memory_1234567890abcdef";
    let secret = "MASTER-SECRET-MEMORY-RECOVERY-TEST";
    let recovery_pass = "recovery-passphrase-mem-88";

    let opts = StorageOptions::default().with_recovery_passphrase(recovery_pass);
    provider.store_secret(bundle, secret, opts).await.unwrap();

    // Recovery payload exists internally
    assert!(provider.get_recovery_payload(bundle).is_some());

    // list_secrets only counts primary
    assert_eq!(provider.list_secrets().await.unwrap().len(), 1);

    // Simulate loss by deleting primary internally
    let wrong_res = provider.recover_secret("nonexistent", recovery_pass, StorageOptions::default()).await;
    assert!(wrong_res.is_err());
    assert!(matches!(wrong_res.unwrap_err(), KnishIOError::SecretNotFound(_)));

    // Wrong recovery passphrase fails
    let wrong_pass_res = provider.recover_secret(bundle, "bad-pass", StorageOptions::default()).await;
    assert!(wrong_pass_res.is_err());
    assert!(matches!(wrong_pass_res.unwrap_err(), KnishIOError::DecryptionFailed(_)));

    // Successful recovery re-enrolls
    provider.recover_secret(bundle, recovery_pass, StorageOptions::default()).await.unwrap();
    let retrieved = provider.retrieve_secret(bundle, StorageOptions::default()).await.unwrap().expect("retrieved");
    assert_eq!(retrieved, secret);

    // Deleting cleans up both
    assert!(provider.delete_secret(bundle).await.unwrap());
    assert!(provider.get_recovery_payload(bundle).is_none());
}
