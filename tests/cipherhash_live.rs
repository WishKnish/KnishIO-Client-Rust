//! Live ML-KEM `CipherHash` encrypted-transport round-trip against a running validator
//! (PQ-transport, Rust SDK).
//!
//! Gated on `CIPHERHASH_TEST_URL` (skips cleanly when unset). Run live against a dev
//! validator:
//!   CIPHERHASH_TEST_URL=http://localhost:8081/graphql cargo test --test cipherhash_live
//!
//! Mirrors the JS reference suite (`tests/cipherhash-live.test.js`): the encrypted
//! client conveys its AUTH wallet's ML-KEM pubkey at auth (a signed `walletPubkey`
//! U-atom meta), the validator decrypts the wrapped request, executes it, and
//! encrypts the response back to that pubkey, which the client decrypts. The
//! transport is transparent, so the encrypted result must equal the plaintext
//! baseline taken on the SAME authenticated session.

use knishio_client::crypto::generate_secret;
use knishio_client::KnishIOClient;

fn test_url() -> Option<String> {
    std::env::var("CIPHERHASH_TEST_URL").ok().filter(|u| !u.is_empty())
}

#[tokio::test]
async fn encrypted_query_round_trips_and_matches_plaintext() {
    let Some(uri) = test_url() else {
        eprintln!(
            "skipping: set CIPHERHASH_TEST_URL (e.g. http://localhost:8081/graphql) to run the \
             live ML-KEM CipherHash round-trip"
        );
        return;
    };

    let secret = generate_secret("cipherhash-live-rust-sdk");

    // ONE authenticated session. Only the transport varies below — the queried
    // wallet stays fixed, so a difference can only come from the envelope.
    let mut client = KnishIOClient::new(
        uri.as_str(),
        Some("public".to_string()),
        None,
        None,
        None,
        Some(false),
    );
    client.set_encrypt(true);
    client
        .request_profile_auth_token(&secret, Some(true))
        .await
        .expect("authenticated session with encryption enabled");

    // Encrypted round-trip: the validator ML-KEM-decrypts the request, executes it,
    // and encrypts the response back to the client's ML-KEM pubkey.
    let encrypted = client
        .query_balance("USER", None)
        .await
        .expect("encrypted queryBalance");

    // Plaintext baseline on the SAME authed session.
    client.set_encrypt(false);
    let plaintext = client
        .query_balance("USER", None)
        .await
        .expect("plaintext queryBalance");

    assert_eq!(
        encrypted.address, plaintext.address,
        "the CipherHash transport must be transparent: the encrypted and plaintext \
         responses for the same wallet must name the same address"
    );
    assert_eq!(
        encrypted.balance, plaintext.balance,
        "the CipherHash transport must be transparent: the encrypted and plaintext \
         responses for the same wallet must carry the same balance"
    );
}
