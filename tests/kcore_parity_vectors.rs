//! kcore parity vectors: full signed molecules, byte-identical with and without kcore.
//!
//! `tests/fixtures/kcore-parity-vectors.json` is a vendored copy of the monorepo master
//! `sdks/shared-test-results/kcore-parity-vectors.json`, produced by
//! `scripts/gen-kcore-parity-vectors.mjs` with the JS reference SDK (kcore off) and cross-checked
//! by re-signing under JS kcore. Sixteen cases: metadata, value transfer, token creation and
//! wallet creation, each under OTS v1 and v2, with unique or colliding `createdAt` values.
//!
//! The suite runs on whichever backend this build links: `cargo test` checks the SDK's own WOTS+
//! loops, `cargo test --features kcore` checks kcore. Both must reproduce every byte.

use knishio_client::molecule::Molecule;
use knishio_client::{Atom, CheckMolecule, OtsVersion, Wallet};
use serde_json::Value;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

/// Every case the master defines; a run over fewer is a failure.
const CASES: usize = 16;

/// A missing fixture fails the test: the vendored copy ships with the crate.
fn cases() -> Result<Vec<Value>, Box<dyn Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/kcore-parity-vectors.json");
    let root: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let tests = root["vectors"]["molecules"]["tests"]
        .as_array()
        .cloned()
        .ok_or("vectors.molecules.tests missing")?;
    assert_eq!(tests.len(), CASES, "kcore parity master case count");
    Ok(tests)
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, Box<dyn Error>> {
    value[field].as_str().ok_or_else(|| format!("field {field} missing").into())
}

fn load(case: &Value) -> Result<Molecule, Box<dyn Error>> {
    Ok(Molecule::fromJSON(&serde_json::to_string(&case["molecule"])?)?)
}

fn ots_version(case: &Value) -> Result<OtsVersion, Box<dyn Error>> {
    match case["otsVersion"].as_u64() {
        Some(1) => Ok(OtsVersion::V1),
        Some(2) => Ok(OtsVersion::V2),
        other => Err(format!("otsVersion {other:?}").into()),
    }
}

fn expected_hash(case: &Value) -> Result<&str, Box<dyn Error>> {
    text(&case["expected"], "molecularHash")
}

fn expected_fragments(case: &Value) -> Result<Vec<Option<String>>, Box<dyn Error>> {
    Ok(case["expected"]["otsFragments"]
        .as_array()
        .ok_or("expected.otsFragments missing")?
        .iter()
        .map(|fragment| fragment.as_str().map(str::to_string))
        .collect())
}

/// The backend under test is the one this build links: kcore with the feature, never without it.
#[test]
fn backend_matches_build() {
    #[cfg(feature = "kcore")]
    assert!(knishio_client::kcore::available(), "kcore feature compiled in but kcore is not available");
    #[cfg(not(feature = "kcore"))]
    assert!(!knishio_client::kcore::available(), "kcore reported available without the kcore feature");
}

/// (a) Each JS-signed molecule hashes to its declared hash and its OTS signature verifies.
#[test]
fn molecules_verify() -> TestResult {
    let mut ran = 0;
    for case in cases()? {
        let name = text(&case, "name")?;
        let molecule = load(&case)?;
        let hash = Atom::hash_atoms(&molecule.atoms, "base17")?;
        assert_eq!(hash, expected_hash(&case)?, "{name}: recomputed molecular hash");
        assert_eq!(molecule.molecular_hash.as_deref(), Some(hash.as_str()), "{name}: declared molecular hash");
        let ots = CheckMolecule::new(&molecule)?.ots();
        assert!(matches!(ots, Ok(true)), "{name}: OTS verification, got {ots:?}");
        ran += 1;
    }
    assert_eq!(ran, CASES, "cases verified");
    Ok(())
}

/// (b) Re-signing each molecule from its secret and source wallet reproduces the molecular hash
/// and every atom's OTS fragment byte for byte.
#[test]
fn resigning_reproduces_js_fragments() -> TestResult {
    let mut ran = 0;
    for case in cases()? {
        let name = text(&case, "name")?;
        let secret = text(&case, "secret")?;
        let mut molecule = load(&case)?;
        molecule.secret = Some(secret.to_string());
        molecule.source_wallet = Some(Wallet::new_with_ots_version(
            Some(secret),
            None,
            Some(text(&case, "sourceToken")?),
            None,
            Some(text(&case, "sourcePosition")?),
            None,
            None,
            None,
            ots_version(&case)?,
        )?);
        // Clear the JS fragments so every compared byte comes from this signing.
        for atom in &mut molecule.atoms {
            atom.ots_fragment = None;
        }
        molecule.sign(None, false, true)?;
        assert_eq!(molecule.molecular_hash.as_deref(), Some(expected_hash(&case)?), "{name}: re-signed molecular hash");
        let actual: Vec<Option<String>> = molecule.atoms.iter().map(|atom| atom.ots_fragment.clone()).collect();
        assert_eq!(actual, expected_fragments(&case)?, "{name}: re-signed fragments");
        ran += 1;
    }
    assert_eq!(ran, CASES, "cases re-signed");
    Ok(())
}
