//! Served-molecule integrity: molecules returned by a validator verify after `from_server_data`.
//!
//! `tests/fixtures/served-molecules.json` was captured from validator 0.8.3 MetaType
//! through-molecule responses and is byte-identical in the JS, TS and Rust SDKs.

use knishio_client::CheckMolecule;
use serde_json::Value;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture() -> Result<Value, Box<dyn Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/served-molecules.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

#[test]
fn served_molecules_verify() -> TestResult {
    let fixture = fixture()?;
    for name in ["v1", "v2"] {
        let result = CheckMolecule::verify_from_server_data(&fixture[name]);
        assert!(result.verified, "{name} must verify, got {result:?}");
        assert!(result.error.is_none(), "{name} error: {:?}", result.error);
        let molecule = CheckMolecule::from_server_data(&fixture[name])?;
        let created: Vec<_> = molecule.atoms.iter().map(|a| a.created_at.clone()).collect();
        println!("{name} createdAt {created:?}");
    }
    Ok(())
}

#[test]
fn server_created_at_normalizes_to_epoch_millis() -> TestResult {
    let fixture = fixture()?;
    let cases: [(Value, &str); 5] = [
        (Value::from("2026-10-14T20:02:37+00:00"), "1792008157000"),
        (Value::from("2026-10-14T20:02:37.110Z"), "1792008157110"),
        (Value::from(1_792_008_157_110_i64), "1792008157110"),
        (Value::from("1792008157110"), "1792008157110"),
        (Value::from("not-a-date"), "not-a-date"),
    ];
    for (input, expected) in cases {
        let mut value = fixture["v1"].clone();
        value["atoms"][0]["createdAt"] = input.clone();
        let molecule = CheckMolecule::from_server_data(&value)?;
        assert_eq!(molecule.atoms[0].created_at.as_str(), expected, "input {input}");
    }
    Ok(())
}
