//! Patent Appendix B: Cross-platform test vector validation
//!
//! Validates the canonical patent test vectors against the Rust SDK crypto functions.
//! These vectors provide reduction-to-practice evidence for patent claims 1-2, 4-5, 8, 12-14, 21.
//!
//! Run: cargo test --test patent_vector_validation

use serde::Deserialize;
use knishio_client::crypto::{
    shake256, generate_bundle_hash, generate_key, generate_address,
    generate_ots_signature, verify_ots_signature,
    hex_to_base17, normalize_hash,
};

// ── JSON structures matching canonical-patent-vectors.json ───────────────

#[derive(Deserialize)]
struct PatentVectors {
    vectors: Vectors,
}

#[derive(Deserialize)]
struct Vectors {
    continuid_chain: Section<ContinuIdTest>,
    base17_enumeration: Section<Base17Test>,
    multi_isotope_molecule: Section<MultiIsotopeTest>,
    bigint_carry_edge: Section<BigIntEdgeTest>,
    wots_roundtrip: Section<WotsRoundtripTest>,
}

#[derive(Deserialize)]
struct Section<T> {
    tests: Vec<T>,
}

// ── ContinuID Chain ─────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContinuIdTest {
    name: String,
    secret: String,
    token: String,
    expected_bundle: String,
    position1: String,
    expected_address1: String,
    expected_position2: String,
    expected_address2: String,
}

// ── Base17 Enumeration ──────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Base17Test {
    name: String,
    hex_input: String,
    expected_base17: String,
    normalized_sum: i32,
}

// ── Multi-Isotope ───────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultiIsotopeTest {
    name: String,
    secret: String,
    expected_bundle: String,
    isotopes: std::collections::HashMap<String, IsotopeSpec>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IsotopeSpec {
    expected_position: String,
    token: String,
    expected_address: String,
}

// ── BigInt Carry Edge ───────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BigIntEdgeTest {
    name: String,
    input: String,
    input_length: usize,
    expected_shake256: String,
    expected_base17_of_hash: String,
    expected_key_length: usize,
}

// ── WOTS+ Roundtrip ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WotsRoundtripTest {
    name: String,
    secret: String,
    token: String,
    position: String,
    expected_ots_address: String,
    molecular_hash_hex: String,
    molecular_hash_base17: String,
    expected_signature_fragment_count: usize,
    expected_signature_fragment0: String,
    expected_signature_fragment15: String,
    expected_verified: bool,
}

// ── Load canonical vectors ──────────────────────────────────────────────

const PATENT_VECTORS_JSON: &str = include_str!(
    "../../../sdks/shared-test-results/canonical-patent-vectors.json"
);

fn load_patent_vectors() -> PatentVectors {
    serde_json::from_str(PATENT_VECTORS_JSON)
        .expect("Failed to parse canonical-patent-vectors.json")
}

// ── Tests ───────────────────────────────────────────────────────────────

/// Patent Claims 5, 12-14: ContinuID identity relay chain
#[test]
fn test_continuid_chain_vectors() {
    let vectors = load_patent_vectors();

    for test in &vectors.vectors.continuid_chain.tests {
        // Verify bundle hash
        let bundle = generate_bundle_hash(&test.secret);
        assert_eq!(bundle, test.expected_bundle,
            "Bundle hash mismatch for '{}'", test.name);

        // Verify wallet at position 1
        let key1 = generate_key(&test.secret, &test.token, &test.position1);
        let address1 = generate_address(&key1).unwrap();
        assert_eq!(address1, test.expected_address1,
            "Address1 mismatch for '{}'", test.name);

        // Verify ContinuID position derivation: position2 = SHAKE256(position1, 256)
        let position2 = shake256(&test.position1, 256);
        assert_eq!(position2, test.expected_position2,
            "Position2 derivation mismatch for '{}'", test.name);

        // Verify wallet at position 2
        let key2 = generate_key(&test.secret, &test.token, &position2);
        let address2 = generate_address(&key2).unwrap();
        assert_eq!(address2, test.expected_address2,
            "Address2 mismatch for '{}'", test.name);

        // Verify invariants
        assert_ne!(test.position1, position2, "Positions must differ");
        assert_ne!(address1, address2, "Addresses must differ");
    }
}

/// Patent Claim 5: Base17 encoding for WOTS+ OTS indexing
#[test]
fn test_base17_enumeration_vectors() {
    let vectors = load_patent_vectors();

    for test in &vectors.vectors.base17_enumeration.tests {
        let base17 = hex_to_base17(&test.hex_input).unwrap();
        assert_eq!(base17, test.expected_base17,
            "Base17 mismatch for '{}'", test.name);

        // Verify normalized sum = 0 (WOTS+ invariant)
        let normalized = normalize_hash(&base17);
        let sum: i32 = normalized.iter().map(|&x| x as i32).sum();
        assert_eq!(sum, test.normalized_sum,
            "Normalized sum mismatch for '{}': expected {}, got {}",
            test.name, test.normalized_sum, sum);
    }
}

/// Patent Claims 8, 21: Multi-isotope molecule composition
#[test]
fn test_multi_isotope_vectors() {
    let vectors = load_patent_vectors();

    for test in &vectors.vectors.multi_isotope_molecule.tests {
        let bundle = generate_bundle_hash(&test.secret);
        assert_eq!(bundle, test.expected_bundle,
            "Bundle mismatch for '{}'", test.name);

        let mut addresses = Vec::new();
        for (isotope_name, spec) in &test.isotopes {
            let key = generate_key(&test.secret, &spec.token, &spec.expected_position);
            let address = generate_address(&key).unwrap();
            assert_eq!(address, spec.expected_address,
                "Address mismatch for '{}' isotope {} at position {}",
                test.name, isotope_name, spec.expected_position);
            addresses.push(address);
        }

        // Verify all addresses are unique (different isotope positions → different wallets)
        let unique_count = {
            let mut sorted = addresses.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(unique_count, addresses.len(),
            "All isotope addresses must be unique for '{}'", test.name);
    }
}

/// Patent Claim 5: BigInt arithmetic edge cases
#[test]
fn test_bigint_carry_edge_vectors() {
    let vectors = load_patent_vectors();

    for test in &vectors.vectors.bigint_carry_edge.tests {
        // Verify input length
        assert_eq!(test.input.len(), test.input_length,
            "Input length mismatch for '{}'", test.name);

        // Verify SHAKE256 hash
        let hash = shake256(&test.input, 256);
        assert_eq!(hash, test.expected_shake256,
            "SHAKE256 mismatch for '{}'", test.name);
        assert_eq!(hash.len(), 64,
            "SHAKE256 output must be 64 hex chars for '{}'", test.name);

        // Verify Base17 of hash
        let base17 = hex_to_base17(&hash).unwrap();
        assert_eq!(base17, test.expected_base17_of_hash,
            "Base17(SHAKE256) mismatch for '{}'", test.name);

        // Verify key generation produces correct length
        let key = generate_key(
            &test.input,
            "USER",
            "0000000000000000000000000000000000000000000000000000000000000001",
        );
        assert_eq!(key.len(), test.expected_key_length,
            "Key length mismatch for '{}'", test.name);
    }
}

/// Patent Claims 1-2, 5: WOTS+ full sign/verify roundtrip
#[test]
fn test_wots_roundtrip_vectors() {
    let vectors = load_patent_vectors();

    for test in &vectors.vectors.wots_roundtrip.tests {
        let key = generate_key(&test.secret, &test.token, &test.position);

        // Compute OTS verification address (16-round hash → join → SHAKE256)
        let mut public_key_fragments = Vec::new();
        for i in 0..16 {
            let start = i * 128;
            let chunk = &key[start..start + 128];
            let mut working = chunk.to_string();
            for _ in 0..16 {
                working = shake256(&working, 512);
            }
            public_key_fragments.push(working);
        }
        let ots_address = shake256(&public_key_fragments.join(""), 256);
        assert_eq!(ots_address, test.expected_ots_address,
            "OTS address mismatch for '{}'", test.name);

        // Verify Base17 conversion of molecular hash
        let base17 = hex_to_base17(&test.molecular_hash_hex).unwrap();
        assert_eq!(base17, test.molecular_hash_base17,
            "Molecular hash Base17 mismatch for '{}'", test.name);

        // Generate signature
        let signature = generate_ots_signature(&key, &test.molecular_hash_base17).unwrap();
        assert_eq!(signature.len(), test.expected_signature_fragment_count,
            "Fragment count mismatch for '{}'", test.name);
        assert_eq!(signature[0], test.expected_signature_fragment0,
            "Fragment 0 mismatch for '{}'", test.name);
        assert_eq!(signature[15], test.expected_signature_fragment15,
            "Fragment 15 mismatch for '{}'", test.name);

        // Verify signature
        let verified = verify_ots_signature(
            &signature,
            &test.molecular_hash_base17,
            &ots_address,
        );
        assert_eq!(verified, test.expected_verified,
            "Verification mismatch for '{}': expected {}", test.name, test.expected_verified);
    }
}
