//! Live OTS version migration against a running validator: an identity created by a client at
//! OTS version 1 moves to version 2 when a client set to version 2 signs in, and stays at
//! version 2 when a client at version 1 signs in afterwards.
//!
//! The test is `#[ignore]`d, so a default `cargo test` reports it as ignored. Run it live with:
//!   OTS_MIGRATION_TEST_URL=http://localhost:8081/graphql cargo test --test ots_migration_live -- --ignored

use std::collections::HashMap;

use knishio_client::crypto::{generate_bundle_hash, generate_secret_with_params};
use knishio_client::response::Response;
use knishio_client::wallet::{OtsVersion, Wallet};
use knishio_client::KnishIOClient;

fn test_url() -> Option<String> {
    std::env::var("OTS_MIGRATION_TEST_URL").ok().filter(|u| !u.is_empty())
}

async fn signed_in(uri: &str, secret: &str, ots_version: OtsVersion) -> KnishIOClient {
    let mut client = KnishIOClient::new(uri, Some("public".to_string()), None, None, None, Some(false));
    client.set_ots_version(ots_version);
    client
        .request_profile_auth_token(secret, Some(false))
        .await
        .expect("authenticated session");
    client
}

/// The identity's USER ContinuID head: (position, address).
async fn head(client: &KnishIOClient, bundle: &str) -> (String, String) {
    let wallet: Wallet = client
        .query_continu_id(Some(bundle), Some("USER"))
        .await
        .expect("ContinuID query")
        .expect("a ContinuID head");
    (wallet.position.expect("head position"), wallet.address.expect("head address"))
}

fn version(secret: &str, head: &(String, String)) -> Option<OtsVersion> {
    OtsVersion::detect(secret, "USER", &head.0, &head.1, OtsVersion::V1)
}

#[tokio::test]
#[ignore = "live: needs OTS_MIGRATION_TEST_URL; run with cargo test --test ots_migration_live -- --ignored"]
async fn a_client_set_to_ots_v2_moves_a_v1_identity_to_v2() {
    let uri = test_url().expect("set OTS_MIGRATION_TEST_URL to run the live OTS migration test");
    let secret = generate_secret_with_params(None, 2048);
    let bundle = generate_bundle_hash(&secret);

    // A client at version 1 creates the identity
    let first = signed_in(&uri, &secret, OtsVersion::V1).await;
    let h1 = head(&first, &bundle).await;
    assert_eq!(version(&secret, &h1), Some(OtsVersion::V1));

    // A client at version 2 signs in from the version 1 head and leaves a version 2 head
    let mut second = signed_in(&uri, &secret, OtsVersion::V2).await;
    let h2 = head(&second, &bundle).await;
    assert_ne!(h2.0, h1.0);
    assert_eq!(version(&secret, &h2), Some(OtsVersion::V2));

    // The version 2 head signs a molecule
    let meta_id = format!("ots-migration-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock").as_millis());
    let meta = second
        .create_meta("ots-migration", &meta_id, HashMap::from([("k".to_string(), serde_json::json!("v"))]), None)
        .await
        .expect("createMeta");
    assert!(meta.success(), "createMeta accepted: {:?}", meta.reason());
    let h3 = head(&second, &bundle).await;
    assert_ne!(h3.0, h2.0);
    assert_eq!(version(&secret, &h3), Some(OtsVersion::V2));

    // A client at version 1 signs in and the identity stays at version 2
    let third = signed_in(&uri, &secret, OtsVersion::V1).await;
    let h4 = head(&third, &bundle).await;
    assert_ne!(h4.0, h3.0);
    assert_eq!(version(&secret, &h4), Some(OtsVersion::V2));
}
