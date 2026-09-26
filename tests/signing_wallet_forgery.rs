//! Regression guard: the verifier must not honour a `signingWallet` atom meta.
//!
//! `tests/fixtures/signing-wallet-forgery.json` was produced by the published JS SDK 1.2.1 and is
//! byte-identical in every KnishIO SDK. `forged` claims the victim's address in `atoms[0]` but
//! carries the attacker's OTS signature and a `signingWallet` meta naming the attacker. A verifier
//! that swaps the expected address for the meta's accepts it and attributes it to the victim.

use knishio_client::error::KnishIOError;
use knishio_client::molecule::Molecule;
use serde_json::Value;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture() -> Result<Value, Box<dyn Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/signing-wallet-forgery.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn load(fixture: &Value, name: &str) -> Result<Molecule, Box<dyn Error>> {
    Ok(Molecule::fromJSON(&serde_json::to_string(&fixture[name])?)?)
}

#[test]
fn genuine_molecule_verifies() -> TestResult {
    let genuine = load(&fixture()?, "genuine")?;
    let result = genuine.check(None);
    assert!(matches!(result, Ok(true)), "genuine molecule must verify, got {result:?}");
    Ok(())
}

#[test]
fn forged_molecule_claiming_victim_address_is_rejected() -> TestResult {
    let fixture = fixture()?;
    let forged = load(&fixture, "forged")?;

    assert_eq!(
        fixture["victimAddress"].as_str(),
        Some(forged.atoms[0].wallet_address.as_str()),
        "forged atoms[0] must claim the victim address"
    );

    let result = forged.check(None);
    assert!(
        matches!(result, Err(KnishIOError::SignatureMismatch)),
        "forged molecule must fail with SignatureMismatch, got {result:?}"
    );
    Ok(())
}
