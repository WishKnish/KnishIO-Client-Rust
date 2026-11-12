//! `{:?}` never prints a secret, passphrase or bearer token (logs, tracing spans and assertion
//! messages format these types with `Debug`).

use knishio_client::auth::{AuthData, AuthToken, AuthTokenSnapshot, WalletSnapshot};
use knishio_client::subscribe::SimpleSubscriptionManager;
use knishio_client::{Molecule, StorageOptions};

const SECRET: &str = "SECRET-0f1e2d3c4b5a";
const PASSPHRASE: &str = "PASSPHRASE-a1b2c3";
const RECOVERY: &str = "RECOVERY-d4e5f6";
const TOKEN: &str = "JWT-eyJhbGciOiJIUzI1NiJ9";

fn assert_redacted(debug: &str, secrets: &[&str]) {
    for secret in secrets {
        assert!(!debug.contains(secret), "Debug output leaks {secret}: {debug}");
    }
    assert!(debug.contains("[REDACTED]"), "Debug output should mark the redacted field: {debug}");
}

#[test]
fn molecule_debug_hides_the_secret() {
    let mut molecule = Molecule::new();
    molecule.secret = Some(SECRET.to_string());
    let debug = format!("{molecule:?}");
    assert_redacted(&debug, &[SECRET]);
    assert!(debug.contains("created_at"), "non-secret fields stay visible: {debug}");
}

#[test]
fn storage_options_debug_hides_both_passphrases() {
    let options = StorageOptions::new(Some("label".to_string()), Some(PASSPHRASE.to_string()))
        .with_recovery_passphrase(RECOVERY);
    let debug = format!("{options:?}");
    assert_redacted(&debug, &[PASSPHRASE, RECOVERY]);
    assert!(debug.contains("label"));
}

#[test]
fn auth_types_debug_hide_the_token() {
    let token = AuthToken::new(TOKEN.to_string(), Some(4_102_444_800), Some(false), None);
    assert_redacted(&format!("{token:?}"), &[TOKEN]);

    let data = AuthData { token: TOKEN.to_string(), pubkey: None, wallet: None };
    assert_redacted(&format!("{data:?}"), &[TOKEN]);

    let snapshot = AuthTokenSnapshot {
        token: TOKEN.to_string(),
        expires_at: None,
        pubkey: None,
        encrypt: None,
        wallet: WalletSnapshot { position: None, characters: None, mlkem_parameter_set: None, token: None, ots_version: None },
    };
    assert_redacted(&format!("{snapshot:?}"), &[TOKEN]);
}

#[test]
fn subscription_manager_debug_hides_the_token() {
    let mut manager = SimpleSubscriptionManager::new();
    manager.set_auth_token(TOKEN.to_string());
    assert_redacted(&format!("{manager:?}"), &[TOKEN]);
}
