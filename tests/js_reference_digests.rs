//! Molecular hashes and helper outputs pinned from the JS reference SDK.
//!
//! Every literal below was printed by the JS reference SDK (`sdks/KnishIO-Client-JS`, built
//! `dist/client.es.mjs`, `KNISHIO_KCORE=off`) from the same pinned inputs: the secret
//! `generateSecret('fleet-audit-2026-11')`, wallet positions `'aa'×32` / `'bb'×32` / `'cc'×32`
//! at OTS v1, and `createdAt = '1700000000000'` written on every atom before hashing with
//! `Atom.hashAtoms`. A Rust builder that diverges from JS changes the digest and fails here.

use knishio_client::crypto::{generate_bundle_hash_with_source, generate_secret, generate_secret_with_params, shake256};
use knishio_client::utils::strings::{charset_base_convert, hex_to_base17};
use knishio_client::{Atom, Isotope, MetaItem, Molecule, OtsVersion, TokenUnit, Version4, Wallet};
use std::collections::HashMap;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const CREATED_AT: &str = "1700000000000";
const POLICY: &str = r#"{"read":{"name":["all"]},"write":{"name":["self"]}}"#;

fn secret() -> String {
    generate_secret("fleet-audit-2026-11")
}

fn position(byte: &str) -> String {
    byte.repeat(32)
}

fn wallet(secret: &str, token: &str, at: &str) -> knishio_client::Result<Wallet> {
    Wallet::new_with_ots_version(Some(secret), None, Some(token), None, Some(&position(at)), None, None, None, OtsVersion::V1)
}

/// A molecule signed by the USER wallet at `aa…`, with the USER remainder at `bb…`.
fn user_molecule(secret: &str) -> knishio_client::Result<Molecule> {
    let mut molecule = Molecule::new();
    molecule.secret = Some(secret.to_string());
    molecule.source_wallet = Some(wallet(secret, "USER", "aa")?);
    molecule.remainder_wallet = Some(wallet(secret, "USER", "bb")?);
    Ok(molecule)
}

/// Pins every atom's `createdAt` and returns the base17 molecular hash.
fn digest(molecule: &mut Molecule) -> knishio_client::Result<String> {
    for atom in &mut molecule.atoms {
        atom.created_at = CREATED_AT.to_string();
    }
    Atom::hash_atoms(&molecule.atoms, "base17")
}

fn meta_json(atom: &Atom) -> Result<String, serde_json::Error> {
    serde_json::to_string(&atom.meta)
}

#[test]
fn identifier_creation_hashes_the_contact_like_js() -> TestResult {
    let secret = secret();
    let mut molecule = user_molecule(&secret)?;
    molecule.init_identifier_creation("email", "  probe@example.com ", "123456")?;

    assert_eq!(
        meta_json(&molecule.atoms[0])?,
        r#"[{"key":"code","value":"123456"},{"key":"hash","value":"3378995cc4dadde1107a04bbedee886ae9e43458b2bad974ee25a995a6bac83d"}]"#
    );
    assert!(!serde_json::to_string(&molecule)?.contains("probe@example.com"), "the contact must not be serialized");
    assert_eq!(digest(&mut molecule)?, "04a82b82e431dcb5d128a8e4c0823cf0689110e5e57d8405dad032570d1d6846");
    Ok(())
}

#[tokio::test]
async fn sign_with_secret_keeps_the_molecular_hash() -> TestResult {
    let secret = secret();
    let mut molecule = user_molecule(&secret)?;
    molecule.init_meta(vec![MetaItem::new("name", "probe")], "probeType", "probe-1", None)?;
    for atom in &mut molecule.atoms {
        atom.created_at = CREATED_AT.to_string();
    }

    molecule.sign_with_secret(&secret).await?;

    let expected = "021f4bd951ba8f8g3g1d03f457b4856913e45d0ga66gec8654f3fc7df81d9886";
    assert_eq!(molecule.molecular_hash.as_deref(), Some(expected));
    assert_eq!(molecule.get_molecular_hash()?, expected);
    let fragment = molecule.atoms[0].ots_fragment.as_deref().ok_or("unsigned")?;
    assert!(!fragment.chars().all(|c| c.is_ascii_hexdigit()), "the signature is compressed (base64), as JS sign() does by default");
    assert!(molecule.check(None)?);
    Ok(())
}

#[test]
fn init_meta_with_a_policy_adds_the_js_policy_atom() -> TestResult {
    let secret = secret();
    let mut molecule = user_molecule(&secret)?;
    molecule.init_meta(vec![MetaItem::new("name", "probe"), MetaItem::new("age", "3")], "probeType", "probe-1", Some(POLICY))?;

    let isotopes: String = molecule.atoms.iter().map(|a| a.isotope.as_str()).collect();
    assert_eq!(isotopes, "MRI");
    assert_eq!(
        meta_json(&molecule.atoms[1])?,
        r#"[{"key":"name","value":"probe"},{"key":"age","value":"3"},{"key":"policy","value":"{\"read\":{\"0\":[\"all\"],\"1\":[\"all\"],\"name\":[\"all\"]},\"write\":{\"0\":[\"self\"],\"1\":[\"self\"],\"name\":[\"self\"]}}"}]"#
    );
    assert_eq!(digest(&mut molecule)?, "05f643dg090dd41dc02bfa8g1d69c12b0d34a79ef3bfdee9b2c90242b2879c3c");
    Ok(())
}

#[test]
fn create_rule_with_a_policy_matches_js_and_is_stable() -> TestResult {
    let secret = secret();
    let rule = r#"[{"condition":[{"key":"name","value":"probe","comparison":"==="}],"callback":[{"action":"reject"}]}]"#;
    for _ in 0..20 {
        let mut molecule = user_molecule(&secret)?;
        molecule.create_rule("probeType", "probe-1", rule, Some(POLICY))?;
        assert_eq!(
            meta_json(&molecule.atoms[0])?,
            r#"[{"key":"rule","value":"[{\"condition\":[{\"key\":\"name\",\"value\":\"probe\",\"comparison\":\"===\"}],\"callback\":[{\"action\":\"reject\"}]}]"},{"key":"policy","value":"{\"read\":{\"0\":[\"all\"],\"name\":[\"all\"]},\"write\":{\"0\":[\"self\"],\"name\":[\"self\"]}}"}]"#
        );
        assert_eq!(digest(&mut molecule)?, "05egfe8d6f86da39g88737174b0ca3faa37a3cc180a99748cfac0fdb9331384g");
    }
    Ok(())
}

#[test]
fn add_policy_atom_signs_from_the_source_wallet() -> TestResult {
    let secret = secret();
    let mut molecule = user_molecule(&secret)?;
    molecule.add_policy_atom("probeType", "probe-1", Vec::new(), Some(POLICY))?;
    molecule.add_continuid_atom()?;

    assert_eq!(molecule.atoms[0].position, position("aa"), "the R atom is signed by the source wallet");
    assert_eq!(
        meta_json(&molecule.atoms[0])?,
        r#"[{"key":"policy","value":"{\"read\":{\"name\":[\"all\"]},\"write\":{\"name\":[\"self\"]}}"}]"#
    );
    assert_eq!(digest(&mut molecule)?, "04dfg6ge3eg5f05cbd6g8dbb91b2180g9c5db7fdag6ab1e4fae590eef978902a");
    Ok(())
}

#[test]
fn deposit_buffer_carries_wallet_metas_like_js() -> TestResult {
    let secret = secret();
    let batch = "dd".repeat(32);
    let mut source = wallet(&secret, "TEST", "aa")?;
    source.balance = "10".to_string();
    source.batch_id = Some(batch.clone());
    source.token_units = vec![
        TokenUnit::new("u1".to_string(), "Unit 1".to_string(), None),
        TokenUnit::new("u2".to_string(), "Unit 2".to_string(), Some(HashMap::from([("color".to_string(), serde_json::json!("red"))]))),
    ];
    let mut remainder = wallet(&secret, "TEST", "cc")?;
    remainder.batch_id = Some(batch);

    let mut molecule = Molecule::new();
    molecule.secret = Some(secret.clone());
    molecule.source_wallet = Some(source);
    molecule.remainder_wallet = Some(remainder);
    molecule.init_deposit_buffer(4.0, HashMap::from([("OTHER".to_string(), 0.5)]))?;

    // The buffer wallet is a fresh random position in both SDKs: pin it to `bb…`.
    let pinned = wallet(&secret, "TEST", "bb")?;
    molecule.atoms[1].position = position("bb");
    molecule.atoms[1].wallet_address = pinned.address.clone().ok_or("no address")?;

    let shape: Vec<(String, Option<String>, String)> = molecule
        .atoms
        .iter()
        .map(|a| Ok((a.isotope.as_str().to_string(), a.value.clone(), meta_json(a)?)))
        .collect::<Result<_, serde_json::Error>>()?;
    assert_eq!(shape, vec![
        ("V".to_string(), Some("-10".to_string()), r#"[{"key":"tokenUnits","value":"[[\"u1\",\"Unit 1\",{}],[\"u2\",\"Unit 2\",{\"color\":\"red\"}]]"}]"#.to_string()),
        ("B".to_string(), Some("4".to_string()), "[]".to_string()),
        ("V".to_string(), Some("6".to_string()), "[]".to_string()),
    ]);
    assert_eq!(digest(&mut molecule)?, "046a7a0c169f43a9a1f83e15g35f6a3cad2b51f10caab0b4d4ba9ed052c862b3");
    Ok(())
}

#[test]
fn string_and_crypto_helpers_return_the_js_values() -> TestResult {
    let secret = secret();
    let hash = shake256("fleet-audit-2026-11", 256);
    assert_eq!(hash, "1e2c25ec65edc9739bab719565abd86951b070be413ec024996c33c5e9e7f84b");
    assert_eq!(hex_to_base17(&hash)?, "00bg4f9ceea06997e078fe643e7868fg46df6g840927578g4a57bd7042a83ga6");
    assert_eq!(
        charset_base_convert(&hash, 16, 17, Some("0123456789abcdef"), Some("0123456789abcdefg")).as_deref(),
        Some("bg4f9ceea06997e078fe643e7868fg46df6g840927578g4a57bd7042a83ga6")
    );
    assert_eq!(
        generate_bundle_hash_with_source(&secret, Some("Molecule::initIdentifierCreation")),
        "d405bf4932289fce650fc090bfbde08f6a712cb38fc2d8414961b455b4337471"
    );
    Ok(())
}

#[test]
fn wallet_helpers_follow_the_js_wallet() -> TestResult {
    let secret = secret();
    let mut keyed = Wallet::new_with_ots_version(None, None, Some("TEST"), None, None, None, None, None, OtsVersion::V2)?;
    keyed.set_key_from_secret(&secret, "TEST", &position("aa"))?;
    let key = keyed.key.as_deref().ok_or("no key")?;
    assert_eq!(shake256(key, 256), "050d2f2b03cafd33cf378eb3e022d6e03620bc3e5207e651911408c1327eefd3", "OTS v2 key");

    let mut source = wallet(&secret, "TEST", "aa")?;
    source.batch_id = Some("dd".repeat(32));
    let mut recipient = Wallet::create(None, Some(&"ee".repeat(32)), "TEST", None, None, None)?;
    recipient.init_batch_id(Some(&source), false);
    let batch_id = recipient.batch_id.as_deref().ok_or("no batch id")?;
    assert_eq!(batch_id.len(), 64, "JS generateBatchId({{}}) is 64 hex characters");
    assert!(batch_id.chars().all(|c| c.is_ascii_hexdigit()));
    Ok(())
}

#[test]
fn large_secrets_squeeze_one_stream() {
    let seed = "fleet-audit-2026-11";
    let standard = generate_secret_with_params(Some(seed), 20000);
    assert_eq!(shake256(&standard, 256), "a4553a5534d7a1061a7a509a6a3005273491856ad7e70c5000c872ef7e9a7c79");
    assert_eq!(knishio_client::crypto::simd::simd_generate_secret_optimized(seed, 20000), standard);
}

#[test]
fn version4_view_matches_the_hashed_structure() -> TestResult {
    let secret = secret();
    let mut first = Atom::new(position("aa"), wallet(&secret, "TEST", "aa")?.address.as_deref().ok_or("no address")?, Isotope::V, "TEST");
    first.value = Some("-10".to_string());
    first.index = Some(0);
    first.created_at = CREATED_AT.to_string();
    first.version = Some("4".to_string());
    let mut second = Atom::new("", "", Isotope::V, "TEST");
    second.value = Some("10".to_string());
    second.batch_id = Some("dd".repeat(32));
    second.meta_type = Some("walletBundle".to_string());
    second.meta_id = Some(position("bb"));
    second.meta = vec![MetaItem::new("a", "1")];
    second.index = Some(1);
    second.created_at = CREATED_AT.to_string();
    second.version = Some("4".to_string());
    let atoms = [first, second];

    let expected = "75a8d98857ce9a09a7088dd541c22def3d8702eeb948695179649b26682fde97";
    assert_eq!(Atom::hash_atoms(&atoms, "hex")?, expected);
    let views: Vec<serde_json::Value> = atoms.iter().map(|a| Version4::from_atom(a).view()).collect();
    assert_eq!(shake256(&serde_json::to_string(&views)?, 256), expected);
    Ok(())
}
