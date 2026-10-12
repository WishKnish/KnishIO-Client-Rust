//! OTS v2 cross-SDK vectors (docs/OTS_V2_SPECIFICATION.md in the monorepo).
//!
//! The master `sdks/shared-test-results/ots-v2-vectors.json` is produced by
//! `scripts/gen-ots-v2-vectors.mjs`: key derivation and signatures come from an independent
//! node:crypto implementation of the specification, ML-KEM identities and molecules from the JS
//! reference SDK. Matching both proves byte parity across three implementations.

// The master lives in the monorepo; build.rs sets `has_ots_v2_vectors` when it exists, and a
// standalone checkout reports the suite as ignored instead of running 0 tests.
#[cfg(not(has_ots_v2_vectors))]
#[test]
#[ignore = "../shared-test-results/ots-v2-vectors.json absent: OTS v2 vector suite not compiled in a standalone checkout"]
fn ots_v2_vectors_absent() {}

#[cfg(has_ots_v2_vectors)]
mod ots_v2 {
    use knishio_client::crypto::{
        generate_address, generate_key, generate_key_v2, generate_ots_signature, verify_ots_signature,
    };
    use knishio_client::error::KnishIOError;
    use knishio_client::CheckMolecule;
    use knishio_client::molecule::Molecule;
    use knishio_client::{MlKemParameterSet, OtsVersion, Wallet};
    use serde_json::Value;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const VECTORS_JSON: &str = include_str!("../../../sdks/shared-test-results/ots-v2-vectors.json");

    fn vectors() -> Result<Value, Box<dyn Error>> {
        let root: Value = serde_json::from_str(VECTORS_JSON)?;
        Ok(root["vectors"].clone())
    }

    fn tests(section: &str) -> Result<Vec<Value>, Box<dyn Error>> {
        vectors()?[section]["tests"]
            .as_array()
            .cloned()
            .ok_or_else(|| format!("vectors.{section}.tests missing").into())
    }

    fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, Box<dyn Error>> {
        value[field].as_str().ok_or_else(|| format!("field {field} missing").into())
    }

    fn key_case(name: &str) -> Result<Value, Box<dyn Error>> {
        tests("key_derivation")?
            .into_iter()
            .find(|k| k["name"] == name)
            .ok_or_else(|| format!("key_derivation {name} missing").into())
    }

    fn molecule_case(name: &str) -> Result<Value, Box<dyn Error>> {
        tests("molecules")?
            .into_iter()
            .find(|m| m["name"] == name)
            .ok_or_else(|| format!("molecule {name} missing").into())
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

    #[test]
    fn key_derivation_and_address() -> TestResult {
        for case in tests("key_derivation")? {
            let name = text(&case, "name")?;
            let (secret, token, position) = (text(&case, "secret")?, text(&case, "token")?, text(&case, "position")?);
            let key_v1 = generate_key(secret, token, position);
            let key_v2 = generate_key_v2(secret, token, position);
            assert_eq!(key_v1, text(&case, "expectedKeyV1")?, "{name}: v1 key");
            assert_eq!(key_v2, text(&case, "expectedKeyV2")?, "{name}: v2 key");
            assert_eq!(generate_address(&key_v1)?, text(&case, "expectedAddressV1")?, "{name}: v1 address");
            assert_eq!(generate_address(&key_v2)?, text(&case, "expectedAddressV2")?, "{name}: v2 address");

            let wallet = Wallet::new_with_ots_version(
                Some(secret), None, Some(token), None, Some(position), None, None, None, OtsVersion::V2,
            )?;
            assert_eq!(wallet.address.as_deref(), Some(text(&case, "expectedAddressV2")?), "{name}: v2 wallet address");
            assert_eq!(wallet.ots_version, OtsVersion::V2);
        }
        Ok(())
    }

    #[test]
    fn signature_over_all_64_symbols() -> TestResult {
        for case in tests("signature")? {
            let name = text(&case, "name")?;
            let hash = text(&case, "molecularHash")?;
            let expected: Vec<i8> = serde_json::from_value(case["expectedNormalized"].clone())?;
            assert_eq!(Molecule::normalize(Molecule::enumerate(hash)), expected, "{name}: normalized hash");

            let key = text(&key_case(text(&case, "keyFrom")?)?, "expectedKeyV2")?.to_string();
            let fragments = generate_ots_signature(&key, hash)?;
            assert_eq!(fragments.len(), 64, "{name}: chain count");
            assert_eq!(fragments.concat(), text(&case, "expectedSignatureHex")?, "{name}: signature");
            assert!(
                verify_ots_signature(&fragments, hash, text(&case, "expectedAddress")?),
                "{name}: the signature must verify against the v2 address"
            );
        }
        Ok(())
    }

    #[test]
    fn mlkem_identity_of_v2_wallets() -> TestResult {
        for case in tests("mlkem_identity")? {
            let name = text(&case, "name")?;
            let key = key_case(text(&case, "keyFrom")?)?;
            let (secret, token, position) = (text(&key, "secret")?, text(&key, "token")?, text(&key, "position")?);
            for (set, field) in [
                (MlKemParameterSet::MlKem1024, "expectedMlKem1024Pubkey"),
                (MlKemParameterSet::MlKem768, "expectedMlKem768Pubkey"),
            ] {
                let wallet = Wallet::create_with_ots_version(
                    Some(secret), None, token, Some(position), None, Some(set), OtsVersion::V2,
                )?;
                assert_eq!(wallet.pubkey.as_deref(), Some(text(&case, field)?), "{name}: {field}");
            }
        }
        Ok(())
    }

    #[test]
    fn molecule_verdicts() -> TestResult {
        for case in tests("molecules")? {
            let name = text(&case, "name")?;
            let result = load(&case)?.check(None);
            match text(&case, "expected")? {
                "verified" => assert!(matches!(result, Ok(true)), "{name}: expected verified, got {result:?}"),
                "SignatureMismatch" => assert!(
                    matches!(result, Err(KnishIOError::SignatureMismatch)),
                    "{name}: expected SignatureMismatch, got {result:?}"
                ),
                "SignatureMalformed" => assert!(
                    matches!(result, Err(KnishIOError::SignatureMalformed)),
                    "{name}: expected SignatureMalformed, got {result:?}"
                ),
                other => return Err(format!("{name}: unknown expectation {other}").into()),
            }
        }
        Ok(())
    }

    /// `CheckMolecule::ots` on its own, without a prior molecular-hash check, must refuse a declared hash that is
    /// not exactly 64 base-17 characters before any chain work.
    #[test]
    fn malformed_hash_is_refused_by_ots() -> TestResult {
        for case in tests("malformed_hash")? {
            let name = text(&case, "name")?;
            assert_eq!(text(&case, "expected")?, "SignatureMalformed", "{name}: expectation");
            let molecule = load(&case)?;
            let result = CheckMolecule::new(&molecule)?.ots();
            assert!(
                matches!(result, Err(KnishIOError::SignatureMalformed)),
                "{name}: expected SignatureMalformed, got {result:?}"
            );
        }
        Ok(())
    }

    /// Re-signing the JS-signed molecules with the Rust SDK must reproduce every fragment.
    #[test]
    fn resigning_reproduces_js_fragments() -> TestResult {
        let key = key_case("hex_secret_user")?;
        let (secret, position) = (text(&key, "secret")?, text(&key, "position")?);
        for name in ["genuine_v2", "genuine_v1_control"] {
            let case = molecule_case(name)?;
            let mut molecule = load(&case)?;
            molecule.secret = Some(secret.to_string());
            molecule.source_wallet = Some(Wallet::new_with_ots_version(
                Some(secret), None, Some("USER"), None, Some(position), None, None, None, ots_version(&case)?,
            )?);
            molecule.sign(None, false, true)?;
            let expected: Vec<Option<String>> = case["molecule"]["atoms"]
                .as_array()
                .ok_or("atoms missing")?
                .iter()
                .map(|atom| atom["otsFragment"].as_str().map(str::to_string))
                .collect();
            let actual: Vec<Option<String>> = molecule.atoms.iter().map(|atom| atom.ots_fragment.clone()).collect();
            assert_eq!(actual, expected, "{name}: re-signed fragments");
        }
        Ok(())
    }
}
