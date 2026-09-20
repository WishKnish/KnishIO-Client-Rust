//! Live ML-KEM `CipherHash` encrypted-transport round-trip against a running validator
//! (PQ-transport, Rust SDK).
//!
//! Gated on `CIPHERHASH_TEST_URL` (skips cleanly when unset). Run live against a dev
//! validator:
//!   CIPHERHASH_TEST_URL=http://localhost:8081/graphql cargo test --test cipherhash_live
//! `CIPHERHASH_MLKEM_PARAMETER_SET=768` selects ML-KEM-768 (default: ML-KEM-1024).
//!
//! Mirrors the JS reference suite (`tests/cipherhash-live.test.js`): the encrypted
//! client conveys its AUTH wallet's ML-KEM pubkey at auth (a signed `walletPubkey`
//! U-atom meta), the validator decrypts the wrapped request, executes it, and
//! encrypts the response back to that pubkey, which the client decrypts.

use knishio_client::crypto::generate_secret_with_params;
use knishio_client::wallet::MlKemParameterSet;
use knishio_client::KnishIOClient;

fn test_url() -> Option<String> {
    std::env::var("CIPHERHASH_TEST_URL").ok().filter(|u| !u.is_empty())
}

fn param_set() -> MlKemParameterSet {
    match std::env::var("CIPHERHASH_MLKEM_PARAMETER_SET").as_deref() {
        Ok("768") => MlKemParameterSet::MlKem768,
        _ => MlKemParameterSet::MlKem1024,
    }
}

fn client_for(uri: &str) -> KnishIOClient {
    let mut client = KnishIOClient::new(uri, Some("public".to_string()), None, None, None, Some(false));
    client.set_mlkem_parameter_set(param_set());
    client
}

/// The transport must be transparent: the same wallet, queried twice on ONE session with
/// only the transport toggled, must come back identical.
///
/// The session authenticates in PLAINTEXT on purpose. The AUTH wallet's ML-KEM pubkey travels
/// as a signed `walletPubkey` U-atom meta regardless of `encrypt`, and the validator's
/// `handle_cipher_hash` keys off that `enc_pubkey` alone — so an `encrypt: false` session still
/// speaks the encrypted transport. Authenticating with `encrypt: true` here would make the
/// plaintext baseline leg a silent downgrade, which an enforcing validator rejects (that
/// rejection is the second case below).
#[tokio::test]
async fn encrypted_query_round_trips_and_matches_plaintext() {
    let Some(uri) = test_url() else {
        eprintln!(
            "skipping: set CIPHERHASH_TEST_URL (e.g. http://localhost:8081/graphql) to run the \
             live ML-KEM CipherHash round-trip"
        );
        return;
    };

    let secret = generate_secret_with_params(None, 2048);
    let mut client = client_for(&uri);
    client
        .request_profile_auth_token(&secret, Some(false))
        .await
        .expect("authenticated session");

    // Encrypted round-trip: the validator ML-KEM-decrypts the request, executes it,
    // and encrypts the response back to the client's ML-KEM pubkey.
    client.switch_encryption(true);
    let encrypted = client
        .query_balance("USER", None)
        .await
        .expect("encrypted queryBalance");

    // Plaintext baseline on the SAME authed session.
    client.switch_encryption(false);
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

/// Live coverage of the issuance-plus-enforcement path: the signed `encrypt` meta →
/// `auth_tokens.encrypted` → `requires_encrypted_transport`. It also proves this SDK's signed
/// literal is the one the validator honours — a JSON-encoded `"true"` (the pre-1.2.0 Rust form)
/// leaves the session plaintext and this case cannot pass.
#[tokio::test]
async fn an_encrypt_true_session_is_refused_when_it_drops_to_plaintext() {
    let Some(uri) = test_url() else {
        eprintln!("skipping: set CIPHERHASH_TEST_URL to run the live enforcement case");
        return;
    };

    let secret = generate_secret_with_params(None, 2048);
    let mut client = client_for(&uri);
    // set_encrypt BEFORE auth: the transport must be encrypting for this session's first query,
    // which is only true when set_encrypt reaches the GraphQL client.
    client.set_encrypt(true);
    client
        .request_profile_auth_token(&secret, Some(true))
        .await
        .expect("authenticated encrypted session");

    // The encrypted transport still works for this session.
    client
        .query_balance("USER", None)
        .await
        .expect("encrypted leg still works");

    // Dropping to plaintext on the same session is the silent downgrade the validator refuses.
    client.set_encrypt(false);
    let err = client
        .query_balance("USER", None)
        .await
        .expect_err("plaintext after encrypt=true must be refused");
    assert!(
        err.to_string()
            .contains("send requests through the CipherHash encrypted transport"),
        "unexpected error: {err}"
    );
}
