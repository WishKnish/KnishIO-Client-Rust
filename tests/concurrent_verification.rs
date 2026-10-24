//! Verification and key derivation give the same answers on many threads at once as on one.
//!
//! WOTS+ chains run through kcore (feature `kcore`) or the pure-Rust SHAKE256 path; neither
//! may keep state that one thread's call can disturb in another's. The batch path (feature
//! `parallel`) must also return its results in input order.

use knishio_client::crypto::{generate_address, generate_key_v2};
use knishio_client::CheckMolecule;
use serde_json::Value;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const SECRET: &str = "a3f1c9e27b40d58e6f12a7c3b9d04e81f5a62c7d3e9b10f4a8c25d6e7f30b19c";
const POSITION: &str = "0c4e8a12f6b3d97e5a21c8f40b6d3e9a7f15c2e8d04b9a6f3e17c5d2a8b04f6e";

/// The four served molecules (all valid) and two tampered copies (both invalid).
fn inputs() -> Result<Vec<Value>, Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/served-molecules.json"))?;
    let mut inputs: Vec<Value> = ["v1", "v2", "v2ts", "v3rust"].iter().map(|k| fixture[*k].clone()).collect();

    let mut metas_tampered = fixture["v2ts"].clone();
    let metas = metas_tampered["atoms"][0]["metasJson"].as_str().ok_or("v2ts carries metasJson")?.to_string();
    metas_tampered["atoms"][0]["metasJson"] = Value::from(metas.replace("f1", "f2"));
    inputs.push(metas_tampered);

    let mut signature_tampered = fixture["v1"].clone();
    let fragment = signature_tampered["atoms"][0]["otsFragment"].as_str().ok_or("v1 carries otsFragment")?.to_string();
    let first = if fragment.starts_with('A') { "B" } else { "A" };
    signature_tampered["atoms"][0]["otsFragment"] = Value::from(format!("{first}{}", &fragment[1..]));
    inputs.push(signature_tampered);

    Ok(inputs)
}

fn verdict(molecule: &Value) -> (bool, Option<String>) {
    let result = CheckMolecule::verify_from_server_data(molecule);
    (result.verified, result.error)
}

#[test]
fn threads_match_sequential() -> TestResult {
    let inputs = inputs()?;
    let expected: Vec<_> = inputs.iter().map(verdict).collect();
    assert_eq!(
        expected.iter().map(|(ok, _)| *ok).collect::<Vec<_>>(),
        [true, true, true, true, false, false],
        "sequential verdicts: {expected:?}"
    );
    assert_eq!(expected[4].1.as_deref(), Some("Molecular hash mismatch"));

    let address = generate_address(&generate_key_v2(SECRET, "USER", POSITION))?;

    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..25 {
                    for (molecule, want) in inputs.iter().zip(&expected) {
                        assert_eq!(&verdict(molecule), want);
                    }
                }
            });
            scope.spawn(|| {
                let derived = generate_address(&generate_key_v2(SECRET, "USER", POSITION)).ok();
                assert_eq!(derived.as_deref(), Some(address.as_str()));
            });
        }
    });
    Ok(())
}

#[cfg(feature = "parallel")]
#[test]
fn batch_preserves_order() -> TestResult {
    let inputs = inputs()?;
    let refs: Vec<&Value> = inputs.iter().cycle().take(256).collect();
    let sequential: Vec<_> = refs.iter().map(|m| verdict(m)).collect();
    let batch: Vec<_> = CheckMolecule::verify_from_server_data_batch(&refs)
        .into_iter()
        .map(|r| (r.verified, r.error))
        .collect();
    assert_eq!(batch, sequential);
    Ok(())
}
