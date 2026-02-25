/// Compute test vectors for patent Appendix B
/// Run with: cargo test --test compute_patent_vectors -- --nocapture
///
/// This generates actual cryptographic outputs for the provisional patent's
/// test vector appendix, replacing [TO BE COMPUTED] placeholders.

use knishio_client::crypto::{generate_key, generate_address, generate_ots_signature};
use knishio_client::atom::Atom;
use knishio_client::types::Isotope;

#[test]
fn compute_tv1_standard_hex_key_derivation() {
    let secret = "d4f5a6b7c8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";
    let token = "KNISH";
    let position = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4d5e6f7a8b9c0d1e2";

    let key = generate_key(secret, token, position);
    let address = generate_address(&key).unwrap();

    println!("=== TEST VECTOR 1: Standard Hex Key Derivation ===");
    println!("K_k length: {} chars", key.len());
    println!("K_k (first 64 hex): {}", &key[..64]);
    println!("wallet_address (64 hex): {}", address);
    println!();

    assert_eq!(key.len(), 2048, "Key must be 2048 hex chars");
    assert_eq!(address.len(), 64, "Address must be 64 hex chars");
}

#[test]
fn compute_tv2_non_hex_secret_normalization() {
    let secret = "my_secret_passphrase";
    let token = "KNISH";
    let position = "0000000000000000000000000000000000000000000000000000000000000001";

    let key = generate_key(secret, token, position);
    let address = generate_address(&key).unwrap();

    println!("=== TEST VECTOR 2: Non-Hex Secret Normalization ===");
    println!("K_k length: {} chars", key.len());
    println!("K_k (first 64 hex): {}", &key[..64]);
    println!("wallet_address (64 hex): {}", address);
    println!();

    assert_eq!(key.len(), 2048);
    assert_eq!(address.len(), 64);
}

#[test]
fn compute_tv3_non_hex_position_normalization() {
    let secret = "d4f5a6b7c8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";
    let token = "GOLD";
    let position = "non_hex_position_string";

    let key = generate_key(secret, token, position);
    let address = generate_address(&key).unwrap();

    println!("=== TEST VECTOR 3: Non-Hex Position Normalization ===");
    println!("K_k length: {} chars", key.len());
    println!("K_k (first 64 hex): {}", &key[..64]);
    println!("wallet_address (64 hex): {}", address);
    println!();

    assert_eq!(key.len(), 2048);
    assert_eq!(address.len(), 64);
}

#[test]
fn compute_tv4_leading_zeros_position() {
    let secret = "d4f5a6b7c8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";
    let token = "KNISH";
    let position = "0000000000000000000000000000000000000000000000000000000000000000";

    let key = generate_key(secret, token, position);
    let address = generate_address(&key).unwrap();

    println!("=== TEST VECTOR 4: Leading Zeros Position ===");
    println!("K_k length: {} chars", key.len());
    println!("K_k (first 64 hex): {}", &key[..64]);
    println!("wallet_address (64 hex): {}", address);
    println!();

    assert_eq!(key.len(), 2048);
    assert_eq!(address.len(), 64);
}

#[test]
fn compute_tv5_bigint_carry() {
    let secret = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let token = "KNISH";
    let position = "0000000000000000000000000000000000000000000000000000000000000001";

    let key = generate_key(secret, token, position);
    let address = generate_address(&key).unwrap();

    println!("=== TEST VECTOR 5: BigInt Carry (65th Character) ===");
    println!("K_k length: {} chars", key.len());
    println!("K_k (first 64 hex): {}", &key[..64]);
    println!("wallet_address (64 hex): {}", address);
    println!();

    assert_eq!(key.len(), 2048);
    assert_eq!(address.len(), 64);
}

#[test]
fn compute_tv6_molecular_hash() {
    // Create 3 V-atoms matching the patent's test vector specification
    let atom0 = Atom {
        position: "aa".repeat(32),
        wallet_address: "bb".repeat(32),
        isotope: Isotope::V,
        token: "KNISH".to_string(),
        value: Some("-100".to_string()),
        batch_id: None,
        meta_type: None,
        meta_id: None,
        meta: vec![],
        ots_fragment: None,
        index: Some(0),
        created_at: "1700000000000".to_string(),
        version: None,
    };

    let atom1 = Atom {
        position: "cc".repeat(32),
        wallet_address: "dd".repeat(32),
        isotope: Isotope::V,
        token: "KNISH".to_string(),
        value: Some("100".to_string()),
        batch_id: None,
        meta_type: None,
        meta_id: None,
        meta: vec![],
        ots_fragment: None,
        index: Some(1),
        created_at: "1700000000000".to_string(),
        version: None,
    };

    let atom2 = Atom {
        position: "ee".repeat(32),
        wallet_address: "bb".repeat(32),
        isotope: Isotope::V,
        token: "KNISH".to_string(),
        value: Some("0".to_string()),
        batch_id: None,
        meta_type: None,
        meta_id: None,
        meta: vec![],
        ots_fragment: None,
        index: Some(2),
        created_at: "1700000000000".to_string(),
        version: None,
    };

    let atoms = vec![atom0, atom1, atom2];

    // hash_atoms with default format returns base17, but for hex we pass "hex"
    let hex_hash = Atom::hash_atoms(&atoms, "hex").unwrap();
    let base17_hash = Atom::hash_atoms(&atoms, "base17").unwrap();

    println!("=== TEST VECTOR 6: Molecular Hash Computation ===");
    println!("molecular_hash (hex, 64 chars): {}", hex_hash);
    println!("molecular_hash (base17, 64 chars): {}", base17_hash);
    println!();

    assert_eq!(hex_hash.len(), 64, "Hex hash must be 64 chars");
    assert_eq!(base17_hash.len(), 64, "Base17 hash must be 64 chars");
}

#[test]
fn compute_tv7_wots_signature() {
    // First compute TV1's key
    let secret = "d4f5a6b7c8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";
    let token = "KNISH";
    let position = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4d5e6f7a8b9c0d1e2";

    let key = generate_key(secret, token, position);
    let address = generate_address(&key).unwrap();

    // Compute TV6's molecular hash (base17 format needed for WOTS+)
    let atom0 = Atom {
        position: "aa".repeat(32),
        wallet_address: "bb".repeat(32),
        isotope: Isotope::V,
        token: "KNISH".to_string(),
        value: Some("-100".to_string()),
        batch_id: None,
        meta_type: None,
        meta_id: None,
        meta: vec![],
        ots_fragment: None,
        index: Some(0),
        created_at: "1700000000000".to_string(),
        version: None,
    };

    let atom1 = Atom {
        position: "cc".repeat(32),
        wallet_address: "dd".repeat(32),
        isotope: Isotope::V,
        token: "KNISH".to_string(),
        value: Some("100".to_string()),
        batch_id: None,
        meta_type: None,
        meta_id: None,
        meta: vec![],
        ots_fragment: None,
        index: Some(1),
        created_at: "1700000000000".to_string(),
        version: None,
    };

    let atom2 = Atom {
        position: "ee".repeat(32),
        wallet_address: "bb".repeat(32),
        isotope: Isotope::V,
        token: "KNISH".to_string(),
        value: Some("0".to_string()),
        batch_id: None,
        meta_type: None,
        meta_id: None,
        meta: vec![],
        ots_fragment: None,
        index: Some(2),
        created_at: "1700000000000".to_string(),
        version: None,
    };

    let atoms = vec![atom0, atom1, atom2];
    let base17_hash = Atom::hash_atoms(&atoms, "base17").unwrap();

    println!("=== TEST VECTOR 7: WOTS+ Signature ===");
    println!("Using K_k from TV1 ({} chars)", key.len());
    println!("Using molecular_hash (base17): {}", base17_hash);

    // Generate WOTS+ signature
    let signature = generate_ots_signature(&key, &base17_hash).unwrap();

    println!("Signature fragment count: {}", signature.len());
    println!("Fragment 0 ({} hex chars): {}", signature[0].len(), &signature[0]);
    println!("TV1 wallet_address: {}", address);
    println!();

    assert_eq!(signature.len(), 16, "Must have 16 OTS fragments");
    assert_eq!(signature[0].len(), 128, "Each fragment must be 128 hex chars");

    // Verify the signature recovers the correct address
    use knishio_client::crypto::verify_ots_signature;
    let verified = verify_ots_signature(&signature, &base17_hash, &address);
    println!("Signature verification against TV1 address: {}", verified);
}

/// Print all test vectors in a consolidated summary
#[test]
fn compute_all_vectors_summary() {
    println!("\n============================================================");
    println!("PATENT APPENDIX B — COMPUTED TEST VECTORS");
    println!("============================================================\n");

    // TV1
    let s1 = "d4f5a6b7c8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";
    let t1 = "KNISH";
    let p1 = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4d5e6f7a8b9c0d1e2";
    let k1 = generate_key(s1, t1, p1);
    let a1 = generate_address(&k1).unwrap();
    println!("TV1 — Standard Hex Key Derivation");
    println!("  K_k first 64: {}", &k1[..64]);
    println!("  wallet_addr:  {}", a1);

    // TV2
    let k2 = generate_key("my_secret_passphrase", "KNISH", "0000000000000000000000000000000000000000000000000000000000000001");
    let a2 = generate_address(&k2).unwrap();
    println!("TV2 — Non-Hex Secret Normalization");
    println!("  K_k first 64: {}", &k2[..64]);
    println!("  wallet_addr:  {}", a2);

    // TV3
    let k3 = generate_key(s1, "GOLD", "non_hex_position_string");
    let a3 = generate_address(&k3).unwrap();
    println!("TV3 — Non-Hex Position Normalization");
    println!("  K_k first 64: {}", &k3[..64]);
    println!("  wallet_addr:  {}", a3);

    // TV4
    let k4 = generate_key(s1, "KNISH", "0000000000000000000000000000000000000000000000000000000000000000");
    let a4 = generate_address(&k4).unwrap();
    println!("TV4 — Leading Zeros Position");
    println!("  K_k first 64: {}", &k4[..64]);
    println!("  wallet_addr:  {}", a4);

    // TV5
    let k5 = generate_key("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", "KNISH", "0000000000000000000000000000000000000000000000000000000000000001");
    let a5 = generate_address(&k5).unwrap();
    println!("TV5 — BigInt Carry");
    println!("  K_k first 64: {}", &k5[..64]);
    println!("  wallet_addr:  {}", a5);

    // TV6 — Molecular hash
    let atoms = vec![
        Atom {
            position: "aa".repeat(32), wallet_address: "bb".repeat(32),
            isotope: Isotope::V, token: "KNISH".to_string(),
            value: Some("-100".to_string()), batch_id: None,
            meta_type: None, meta_id: None, meta: vec![],
            ots_fragment: None, index: Some(0),
            created_at: "1700000000000".to_string(), version: None,
        },
        Atom {
            position: "cc".repeat(32), wallet_address: "dd".repeat(32),
            isotope: Isotope::V, token: "KNISH".to_string(),
            value: Some("100".to_string()), batch_id: None,
            meta_type: None, meta_id: None, meta: vec![],
            ots_fragment: None, index: Some(1),
            created_at: "1700000000000".to_string(), version: None,
        },
        Atom {
            position: "ee".repeat(32), wallet_address: "bb".repeat(32),
            isotope: Isotope::V, token: "KNISH".to_string(),
            value: Some("0".to_string()), batch_id: None,
            meta_type: None, meta_id: None, meta: vec![],
            ots_fragment: None, index: Some(2),
            created_at: "1700000000000".to_string(), version: None,
        },
    ];
    let hex_hash = Atom::hash_atoms(&atoms, "hex").unwrap();
    let base17_hash = Atom::hash_atoms(&atoms, "base17").unwrap();
    println!("TV6 — Molecular Hash");
    println!("  hex hash:     {}", hex_hash);
    println!("  base17 hash:  {}", base17_hash);

    // TV7 — WOTS+ signature
    let sig = generate_ots_signature(&k1, &base17_hash).unwrap();
    println!("TV7 — WOTS+ Signature");
    println!("  fragment 0:   {}", &sig[0]);
    println!("  frag0 length: {} hex chars", sig[0].len());

    let verified = knishio_client::crypto::verify_ots_signature(&sig, &base17_hash, &a1);
    println!("  verified:     {}", verified);

    println!("\n============================================================");
}
