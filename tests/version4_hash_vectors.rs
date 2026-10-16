//! Atom-hash vectors shared by every SDK.
//!
//! `tests/fixtures/version4-hash-vectors.json` is a vendored copy of the master produced by
//! the monorepo's `scripts/gen-version4-hash-vectors.mjs`. Each versioned case (`cases`)
//! gives client-format atoms, the exact JSON string hashed and the expected base17
//! molecular hash; each legacy case (`legacyCases`) gives atoms without a version and their
//! expected legacy molecular hash.

use knishio_client::types::AtomFromJsonOptions;
use knishio_client::Atom;
use serde_json::Value;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const CASES: [&str; 9] = [
    "v4-basic",
    "v3-basic",
    "slash-unicode",
    "control-quote",
    "line-separators",
    "nulls",
    "empty-strings",
    "empty-version",
    "index-order",
];

const LEGACY_CASES: [&str; 2] = ["legacy-empty-strings", "legacy-nulls"];

fn fixture() -> Result<Value, Box<dyn Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/version4-hash-vectors.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn names(fixture: &Value, group: &str) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(fixture[group]
        .as_array()
        .ok_or_else(|| format!("{group} missing"))?
        .iter()
        .filter_map(|c| c["name"].as_str().map(str::to_string))
        .collect())
}

/// Parse the case's atoms with `Atom::from_json` and compare `Atom::hash_atoms` with
/// `expectedHash`. Returns the parsed atoms.
fn check(group: &str, name: &str) -> Result<Vec<Atom>, Box<dyn Error>> {
    let fixture = fixture()?;
    let vector = fixture[group]
        .as_array()
        .ok_or_else(|| format!("{group} missing"))?
        .iter()
        .find(|c| c["name"] == name)
        .ok_or_else(|| format!("case {name} missing"))?;
    let atoms = vector["atoms"]
        .as_array()
        .ok_or("atoms missing")?
        .iter()
        .map(|json| Atom::from_json(json, AtomFromJsonOptions::default()))
        .collect::<Result<Vec<_>, _>>()?;
    let expected = vector["expectedHash"].as_str().ok_or("expectedHash missing")?;
    assert_eq!(Atom::hash_atoms(&atoms, "base17")?, expected, "case {name}");
    Ok(atoms)
}

fn case(name: &str) -> TestResult {
    check("cases", name)?;
    Ok(())
}

fn legacy_case(name: &str) -> TestResult {
    let atoms = check("legacyCases", name)?;
    assert!(atoms.iter().all(|a| a.version.is_none()), "{name}: legacy atoms carry no version");
    Ok(())
}

#[test]
fn fixture_carries_the_nine_cases() -> TestResult {
    assert_eq!(names(&fixture()?, "cases")?, CASES);
    Ok(())
}

#[test]
fn fixture_carries_the_two_legacy_cases() -> TestResult {
    assert_eq!(names(&fixture()?, "legacyCases")?, LEGACY_CASES);
    Ok(())
}

#[test]
fn v4_basic() -> TestResult {
    case("v4-basic")
}

#[test]
fn v3_basic() -> TestResult {
    case("v3-basic")
}

#[test]
fn slash_unicode() -> TestResult {
    case("slash-unicode")
}

#[test]
fn control_quote() -> TestResult {
    case("control-quote")
}

#[test]
fn line_separators() -> TestResult {
    case("line-separators")
}

#[test]
fn nulls() -> TestResult {
    case("nulls")
}

#[test]
fn empty_strings() -> TestResult {
    case("empty-strings")
}

#[test]
fn empty_version() -> TestResult {
    case("empty-version")
}

#[test]
fn index_order() -> TestResult {
    case("index-order")
}

#[test]
fn legacy_empty_strings() -> TestResult {
    legacy_case("legacy-empty-strings")
}

#[test]
fn legacy_nulls() -> TestResult {
    legacy_case("legacy-nulls")
}
