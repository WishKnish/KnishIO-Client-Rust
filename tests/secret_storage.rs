use knishio_client::client::KnishIOClient;
use knishio_client::crypto::{generate_bundle_hash, generate_secret};
use knishio_client::error::KnishIOError;
use knishio_client::storage::secure_memory::{constant_time_equals, with_secure_bytes, zeroize_bytes};
use knishio_client::storage::{
    AesGcmSecretStorageProvider, MemorySecretStorageProvider, SecretStorageProvider,
    StorageOptions,
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
        false,
    );

    assert!(provider.is_available().await);
    assert_eq!(provider.provider_type(), "aes-gcm");

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
        false,
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
