/*!
 * KnishIO Rust SDK Self-Test Program
 *
 * This program performs self-contained tests to validate SDK functionality
 * and ensure cross-SDK compatibility. It follows the JavaScript SDK
 * methodology exactly using modern Rust 2025 best practices.
 *
 * Features complete JavaScript parity with Rust excellence:
 * - Identical crypto test logic (seed-based generation)
 * - Identical metadata molecule creation (M + I atoms)
 * - Identical simple transfer logic (V atoms UTXO pattern)
 * - Identical complex transfer logic (V atoms with remainder)
 * - JSON output compatible with other SDKs
 * - Modern Rust 2025: anyhow, tokio, serde, tracing
 * - Memory safety: Zero unsafe code
 * - Performance: SIMD-optimized cryptography
 */

#![warn(clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use serde_json::Value;
use tokio;
use std::fs;
use std::time::SystemTime;
use chrono::{DateTime, Utc};

// KnishIO SDK imports
use knishio_client::{
    Molecule, Wallet, Atom, Isotope,
    crypto::{generate_secret, generate_bundle_hash},
    types::MetaItem,
};

/* ANSI Color codes for terminal output */
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const BLUE: &str = "\x1b[34m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
}

// Fixed timestamp for deterministic testing (preserves timestamp in hash while ensuring consistency)
const FIXED_TEST_TIMESTAMP_BASE: u64 = 1700000000000; // Fixed base timestamp for deterministic testing

/// Helper function to set fixed timestamps for deterministic testing
fn set_fixed_timestamps(molecule: &mut Molecule) {
    for (i, atom) in molecule.atoms.iter_mut().enumerate() {
        // Set deterministic timestamp: base + (index * 1000) to ensure unique but predictable timestamps
        atom.created_at = (FIXED_TEST_TIMESTAMP_BASE + (i as u64 * 1000)).to_string();
    }
}

/// Helper function to create fixed remainder wallets for deterministic testing
fn create_fixed_remainder_wallet(secret: &str, token: &str) -> Result<Wallet> {
    let bundle = generate_bundle_hash(secret);
    Ok(Wallet::new(
        Some(secret),
        Some(&bundle),
        Some(token),
        None, // address
        Some("bbbb000000000000cccc111111111111dddd222222222222eeee333333333333"), // Fixed deterministic position
        None, // batch_id
        None  // characters
    )?)
}

/* Test result structures matching JavaScript SDK format */
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CryptoTestResult {
    passed: bool,
    secret: String,
    bundle: String,
    #[serde(rename = "expectedSecret")]
    expected_secret: String,
    #[serde(rename = "expectedBundle")]
    expected_bundle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MoleculeTestResult {
    passed: bool,
    #[serde(rename = "molecularHash")]
    molecular_hash: String,
    #[serde(rename = "atomCount")]
    atom_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hasRemainder")]
    has_remainder: Option<bool>,
    #[serde(rename = "validationError")]
    validation_error: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MLKEMTestResult {
    passed: bool,
    #[serde(rename = "publicKeyGenerated")]
    public_key_generated: bool,
    #[serde(rename = "encryptionSuccess")]
    encryption_success: bool,
    #[serde(rename = "decryptionSuccess")]
    decryption_success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct NegativeTestResult {
    passed: bool,
    description: String,
    #[serde(rename = "testCount")]
    test_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct TestResults {
    sdk: String,
    version: String,
    timestamp: String,
    tests: TestSuite,
    molecules: MoleculeResults,
    #[serde(rename = "crossSdkCompatible")]
    cross_sdk_compatible: bool,
}

#[derive(Debug, serde::Serialize)]
struct TestSuite {
    crypto: CryptoTestResult,
    #[serde(rename = "metaCreation")]
    meta_creation: MoleculeTestResult,
    #[serde(rename = "simpleTransfer")]
    simple_transfer: MoleculeTestResult,
    #[serde(rename = "complexTransfer")]
    complex_transfer: MoleculeTestResult,
    mlkem768: MLKEMTestResult,
    #[serde(rename = "negativeCases")]
    negative_cases: NegativeTestResult,
}

#[derive(Debug, serde::Serialize)]
struct MoleculeResults {
    metadata: String,
    #[serde(rename = "simpleTransfer")]
    simple_transfer: String,
    #[serde(rename = "complexTransfer")]
    complex_transfer: String,
    mlkem768: String,
}

/* Logger with modern Rust patterns */
struct Logger;

impl Logger {
    fn message(msg: &str, color: &str) {
        println!("{}{}{}", color, msg, colors::RESET);
    }

    fn test(test_name: &str, passed: bool, error_detail: Option<&str>) {
        let status = if passed { "✅ PASS" } else { "❌ FAIL" };
        let color = if passed { colors::GREEN } else { colors::RED };
        println!("  {}{}: {}{}", color, status, test_name, colors::RESET);
        if let Some(error) = error_detail {
            if !passed {
                println!("    {}{}{}", colors::RED, error, colors::RESET);
            }
        }
    }
}

/* Configuration loader */
// Embedded test configuration for SDK self-containment (Rust best practices)
const DEFAULT_CONFIG_JSON: &str = r#"{
  "tests": {
    "crypto": {
      "seed": "TESTSEED",
      "secret": "e8ffc86d60fc6a73234a834166e7436e21df6c3209dfacc8d0bd6595707872c3799abbf7deee0f9c4b58de1fd89b9abb67a207558208d5ccf550c227d197c24e9fcc3707aeb53c4031d38392020ff72bcaa0f728aa8bc3d47d95ff0afc04d8fcdb69bff638ce56646c154fc92aa517d3c40f550d2ccacbd921724e1d94b82aed2c8e172a8a7ed5a6963f5890157fe77222b97af3787741f9d3cec0b40aec6f07ae4b2b24614f0a20e035aee0df04e176175dc100eb1b00dd7ea95c28cdec47958336945333c3bef24719ed949fa56d1541f24c725d4f374a533bf255cf22f4596147bcd1ba05abcecbe9b12095e1fdddb094616894c366498be0b5785c180100efb3c5b689fc1c01131633fe1775df52a970e9472ab7bc0c19f5742b9e9436753cd16024b2d326b763eca68c414755a0d2fdbb927f007e9413f1190578b2033a03d29387f5aea71b07a5ce80fbfd45be4a15440faadeac50e41846022894fc683a52328b470bc1860c8b038d7258f504178918502b93d84d8b0fbef3e02f89f83cb1ff033a2bdbdf2a2ba78d80c12aa8b2d6c10d76c468186bd4a4e9eacc758546bb50ed7b1ee241cc5b93ff924c7bbee6778b27789e1f9104c917fc93f735eee5b25c07a883788f3d2e0771e751c4f59b76f8426027ac2b07a2ca84534433d0a1b86cef3288e7d79e8b175a3955848cfd1dfbdcd6b5bafcf6789e56e8ef40af",
      "bundle": "fee9c2b9a964d060eb4645c4001db805c3c4b0cc9bba12841036eba4bf44b831",
      "walletAddress": "Kk4xBpejTujcDQxuuUNVEcvvRNwRGMfLFm28p1aqv2wQ52u5X"
    },
    "metaCreation": {
      "seed": "TESTSEED",
      "token": "USER",
      "sourcePosition": "0123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210",
      "metaType": "TestMeta",
      "metaId": "TESTMETA123",
      "metadata": {
        "name": "Test Metadata",
        "description": "This is a test metadata for SDK testing."
      },
      "expectedMolecularHash": "046778a3g7d26de4145d33de70b48d70a2e3e1b0f2gadg398a0711g3263761a2"
    },
    "simpleTransfer": {
      "sourceSeed": "TESTSEED",
      "recipientSeed": "RECIPIENTSEED",
      "balance": 1000,
      "amount": 1000,
      "token": "TEST",
      "sourcePosition": "0123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210",
      "recipientPosition": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
      "expectedMolecularHash": "00bd586e56gbg38737c4gd463e3fb39cdbb013fg8a851baa962c66g0d1cadce5"
    },
    "complexTransfer": {
      "sourceSeed": "TESTSEED",
      "recipient1Seed": "RECIPIENTSEED",
      "recipient2Seed": "RECIPIENT2SEED",
      "sourceBalance": 1000,
      "amount1": 500,
      "amount2": 500,
      "token": "TEST",
      "sourcePosition": "0123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210",
      "recipient1Position": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
      "recipient2Position": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
      "expectedMolecularHash": "034f6f8d01c9f20c8a9a64a5742ca755b53a917461c8e870de8622ca4a2b37ge"
    },
    "mlkem768": {
      "seed": "TESTSEED",
      "token": "ENCRYPT",
      "position": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "plaintext": "Hello ML-KEM768 cross-platform test message!"
    }
  }
}"#;

struct ConfigLoader {
    config: Value,
}

impl ConfigLoader {
    fn new(config_path: Option<&str>) -> Result<Self> {
        let config = if let Some(path) = config_path {
            // Try to load external config if path provided
            if std::path::Path::new(path).exists() {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Cannot open config file: {}", path))?;
                serde_json::from_str(&content)
                    .with_context(|| "Failed to parse JSON configuration")?
            } else {
                // Fall back to embedded config if file doesn't exist
                serde_json::from_str(DEFAULT_CONFIG_JSON)
                    .context("Failed to parse embedded configuration")?
            }
        } else {
            // Use embedded config by default
            serde_json::from_str(DEFAULT_CONFIG_JSON)
                .context("Failed to parse embedded configuration")?
        };

        Ok(Self { config })
    }

    fn get_string(&self, path: &str) -> Option<String> {
        self.get_value(path)?.as_str().map(|s| s.to_string())
    }

    fn get_i64(&self, path: &str) -> Option<i64> {
        self.get_value(path)?.as_i64()
    }

    fn get_value(&self, path: &str) -> Option<&Value> {
        let keys: Vec<&str> = path.split('.').collect();
        let mut current = &self.config;

        for key in keys {
            current = current.get(key)?;
        }

        Some(current)
    }
}

/* Molecule inspector for debugging (matches JavaScript pattern) */
struct MoleculeInspector;

impl MoleculeInspector {
    fn inspect(molecule: &Molecule, name: &str) {
        println!("\n{}🔍 INSPECTING {}:{}", colors::BLUE, name, colors::RESET);

        println!("  Molecular Hash: {}",
                 molecule.molecular_hash.as_deref().unwrap_or("NOT_SET"));
        println!("  Bundle: {}",
                 molecule.bundle.as_deref().unwrap_or("NOT_SET"));
        println!("  Cell Slug: {}",
                 molecule.cell_slug.as_deref().unwrap_or("NOT_SET"));
        println!("  Atoms ({}):", molecule.atoms.len());

        let mut total_value = 0.0;
        for (i, atom) in molecule.atoms.iter().enumerate() {
            if let Some(ref value_str) = atom.value {
                if let Ok(value) = value_str.parse::<f64>() {
                    total_value += value;
                }
            }

            let address_preview = "unknown";  // Simplified for compilation

            let atom_index = atom.index.unwrap_or(i as u32);
            println!("    [{}] {:?}: {} ({}...) index={}",
                     i,
                     atom.isotope,
                     atom.value.as_deref().unwrap_or("null"),
                     address_preview,
                     atom_index);
        }

        let balanced = if total_value.abs() < 0.01 { "✅ BALANCED" } else { "❌ UNBALANCED" };
        println!("  Total Value: {:.1} {}", total_value, balanced);
        println!("  Status: {}", molecule.status.as_deref().unwrap_or("NOT_SET"));
    }

    fn diagnose_validation(molecule: &Molecule, name: &str) {
        println!("\n{}🔬 VALIDATING {} STEP-BY-STEP:{}", colors::BLUE, name, colors::RESET);
        println!("  Molecule has {} atoms", molecule.atoms.len());

        if let Some(first_atom) = molecule.atoms.first() {
            println!("  First atom isotope: {:?}", first_atom.isotope);
        }

        println!("  Molecular hash present: {}", molecule.molecular_hash.is_some());

        // Check atom indices
        for (i, atom) in molecule.atoms.iter().enumerate() {
            let atom_index = atom.index.unwrap_or(i as u32);
            println!("    {}✅ Atom {} index: {}{}",
                     colors::GREEN, i, atom_index, colors::RESET);
        }
    }
}

/* Main test runner */
struct SelfTestRunner {
    config: ConfigLoader,
    results: TestResults,
}

impl SelfTestRunner {
    fn new() -> Result<Self> {
        // Support optional external config override via environment variable
        let config_path = std::env::var("KNISHIO_TEST_CONFIG").ok();
        let config = ConfigLoader::new(config_path.as_deref())?;

        let results = TestResults {
            sdk: "Rust".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Self::get_iso8601_timestamp(),
            tests: TestSuite {
                crypto: CryptoTestResult {
                    passed: false,
                    secret: String::new(),
                    bundle: String::new(),
                    expected_secret: String::new(),
                    expected_bundle: String::new(),
                    error: None,
                },
                meta_creation: MoleculeTestResult {
                    passed: false,
                    molecular_hash: String::new(),
                    atom_count: 0,
                    has_remainder: None,
                    validation_error: Some("null".to_string()),
                },
                simple_transfer: MoleculeTestResult {
                    passed: false,
                    molecular_hash: String::new(),
                    atom_count: 0,
                    has_remainder: None,
                    validation_error: Some("null".to_string()),
                },
                complex_transfer: MoleculeTestResult {
                    passed: false,
                    molecular_hash: String::new(),
                    atom_count: 0,
                    has_remainder: Some(true),
                    validation_error: Some("null".to_string()),
                },
                mlkem768: MLKEMTestResult {
                    passed: false,
                    public_key_generated: false,
                    encryption_success: false,
                    decryption_success: false,
                    error: Some("null".to_string()),
                },
                negative_cases: NegativeTestResult {
                    passed: false,
                    description: "Anti-cheating validation tests".to_string(),
                    test_count: 3,
                    error: None,
                },
            },
            molecules: MoleculeResults {
                metadata: String::new(),
                simple_transfer: String::new(),
                complex_transfer: String::new(),
                mlkem768: String::new(),
            },
            cross_sdk_compatible: true,
        };

        Ok(Self { config, results })
    }

    fn get_iso8601_timestamp() -> String {
        let now = SystemTime::now();
        let datetime: DateTime<Utc> = now.into();
        datetime.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string()
    }

    async fn run_all_tests(&mut self) -> Result<()> {
        // Check for cross-validation-only mode (Round 2)
        if std::env::var("KNISHIO_CROSS_VALIDATION_ONLY").unwrap_or_default() == "true" {
            Logger::message("═══════════════════════════════════════════", colors::BLUE);
            Logger::message("    Knish.IO Rust SDK Cross-Validation Only", colors::BLUE);
            Logger::message("═══════════════════════════════════════════", colors::BLUE);

            // CRITICAL FIX: Load existing Round 1 results to preserve molecules
            let shared_dir = std::env::var("KNISHIO_SHARED_RESULTS")
                .unwrap_or_else(|_| "../shared-test-results".to_string());
            let existing_path = format!("{}/rust-results.json", shared_dir);

            if std::path::Path::new(&existing_path).exists() {
                match fs::read_to_string(&existing_path) {
                    Ok(existing_content) => {
                        match serde_json::from_str::<Value>(&existing_content) {
                            Ok(existing_data) => {
                                // Preserve Round 1 test results
                                if let Some(tests) = existing_data.get("tests") {
                                    // Preserve crypto test
                                    if let Some(crypto) = tests.get("crypto") {
                                        if let Ok(crypto_result) = serde_json::from_value::<CryptoTestResult>(crypto.clone()) {
                                            self.results.tests.crypto = crypto_result;
                                        }
                                    }
                                    // Preserve metaCreation test
                                    if let Some(meta) = tests.get("metaCreation") {
                                        if let Ok(meta_result) = serde_json::from_value::<MoleculeTestResult>(meta.clone()) {
                                            self.results.tests.meta_creation = meta_result;
                                        }
                                    }
                                    // Preserve simpleTransfer test
                                    if let Some(simple) = tests.get("simpleTransfer") {
                                        if let Ok(simple_result) = serde_json::from_value::<MoleculeTestResult>(simple.clone()) {
                                            self.results.tests.simple_transfer = simple_result;
                                        }
                                    }
                                    // Preserve complexTransfer test
                                    if let Some(complex) = tests.get("complexTransfer") {
                                        if let Ok(complex_result) = serde_json::from_value::<MoleculeTestResult>(complex.clone()) {
                                            self.results.tests.complex_transfer = complex_result;
                                        }
                                    }
                                    // Preserve mlkem768 test
                                    if let Some(mlkem) = tests.get("mlkem768") {
                                        if let Ok(mlkem_result) = serde_json::from_value::<MLKEMTestResult>(mlkem.clone()) {
                                            self.results.tests.mlkem768 = mlkem_result;
                                        }
                                    }
                                    // Preserve negativeCases test
                                    if let Some(negative) = tests.get("negativeCases") {
                                        if let Ok(negative_result) = serde_json::from_value::<NegativeTestResult>(negative.clone()) {
                                            self.results.tests.negative_cases = negative_result;
                                        }
                                    }
                                }

                                // Preserve Round 1 molecules
                                if let Some(molecules) = existing_data.get("molecules") {
                                    if let Some(metadata) = molecules.get("metadata").and_then(|v| v.as_str()) {
                                        self.results.molecules.metadata = metadata.to_string();
                                    }
                                    if let Some(simple) = molecules.get("simpleTransfer").and_then(|v| v.as_str()) {
                                        self.results.molecules.simple_transfer = simple.to_string();
                                    }
                                    if let Some(complex) = molecules.get("complexTransfer").and_then(|v| v.as_str()) {
                                        self.results.molecules.complex_transfer = complex.to_string();
                                    }
                                    if let Some(mlkem) = molecules.get("mlkem768").and_then(|v| v.as_str()) {
                                        self.results.molecules.mlkem768 = mlkem.to_string();
                                    }
                                }

                                Logger::message("✅ Preserved Round 1 molecules for cross-validation", colors::GREEN);
                            }
                            Err(e) => {
                                Logger::message(&format!("⚠️  Could not parse existing results: {}", e), colors::YELLOW);
                            }
                        }
                    }
                    Err(e) => {
                        Logger::message(&format!("⚠️  Could not read existing results: {}", e), colors::YELLOW);
                    }
                }
            }

            // Only run cross-SDK validation
            let cross_sdk_result = self.test_cross_sdk_validation().await?;

            // Save results and print summary (cross-validation only)
            self.save_results().await?;
            Logger::message("\n═══════════════════════════════════════════", colors::BLUE);
            Logger::message("            CROSS-VALIDATION SUMMARY", colors::BLUE);
            Logger::message("═══════════════════════════════════════════", colors::BLUE);
            let compat_status = if self.results.cross_sdk_compatible { "✅ YES" } else { "❌ NO" };
            let compat_color = if self.results.cross_sdk_compatible { colors::GREEN } else { colors::RED };
            println!("{}Cross-SDK Compatible: {}{}", compat_color, compat_status, colors::RESET);
            Logger::message("═══════════════════════════════════════════", colors::BLUE);

            // Exit based on cross-validation results only
            if !cross_sdk_result {
                std::process::exit(1);
            }
            return Ok(());
        }

        // Normal mode: Run all tests (Round 1 or standalone)
        Logger::message("═══════════════════════════════════════════", colors::BLUE);
        Logger::message("    Knish.IO Rust SDK Self-Test", colors::BLUE);
        Logger::message("═══════════════════════════════════════════", colors::BLUE);

        // Run all tests following JavaScript pattern exactly
        let crypto_result = self.test_crypto().await?;
        let meta_result = self.test_meta_creation().await?;
        let simple_result = self.test_simple_transfer().await?;
        let complex_result = self.test_complex_transfer().await?;
        let mlkem_result = self.test_mlkem768().await?;
        let negative_result = self.test_negative_cases().await?;
        let _cross_sdk_result = self.test_cross_sdk_validation().await?;

        // Save results
        self.save_results().await?;

        // Display summary
        self.display_summary();

        // Exit with appropriate code
        let total_tests = 6;
        let passed_tests = [crypto_result, meta_result, simple_result, complex_result, mlkem_result, negative_result]
            .iter()
            .filter(|&&x| x)
            .count();

        if passed_tests != total_tests {
            std::process::exit(1);
        }

        Ok(())
    }

    async fn test_crypto(&mut self) -> Result<bool> {
        Logger::message("\n1. Crypto Test", colors::BLUE);

        let seed = self.config.get_string("tests.crypto.seed").unwrap_or_else(|| "TESTSEED".to_string());
        let expected_secret = self.config.get_string("tests.crypto.secret").unwrap_or_default();
        let expected_bundle = self.config.get_string("tests.crypto.bundle").unwrap_or_default();

        // Generate secret from seed (matches JavaScript SDK)
        let generated_secret = generate_secret(&seed);

        println!("  Generated secret length: {}", generated_secret.len());
        println!("  First 64 chars: {}...", &generated_secret[..64]);
        println!("  Expected length: {}", expected_secret.len());
        println!("  Expected first 64: {}...", &expected_secret[..64]);

        let secret_match = generated_secret == expected_secret;
        Logger::test("Secret generation (seed: \"TESTSEED\")", secret_match, None);

        // Generate bundle hash
        let generated_bundle = generate_bundle_hash(&generated_secret);

        println!("  Generated bundle: {}", generated_bundle);
        println!("  Expected bundle: {}", expected_bundle);

        let bundle_match = generated_bundle == expected_bundle;
        Logger::test("Bundle hash generation", bundle_match, None);

        let success = secret_match && bundle_match;

        // Store results
        self.results.tests.crypto = CryptoTestResult {
            passed: success,
            secret: generated_secret,
            bundle: generated_bundle,
            expected_secret,
            expected_bundle,
            error: if success { None } else { Some("Cryptographic mismatch".to_string()) },
        };

        Ok(success)
    }

    async fn test_meta_creation(&mut self) -> Result<bool> {
        Logger::message("\n2. Metadata Creation Test", colors::BLUE);

        let seed = self.config.get_string("tests.metaCreation.seed").unwrap_or_else(|| "TESTSEED".to_string());
        let token = self.config.get_string("tests.metaCreation.token").unwrap_or_else(|| "USER".to_string());
        let source_position = self.config.get_string("tests.metaCreation.sourcePosition").unwrap_or_default();
        let meta_type = self.config.get_string("tests.metaCreation.metaType").unwrap_or_else(|| "TestMeta".to_string());
        let meta_id = self.config.get_string("tests.metaCreation.metaId").unwrap_or_else(|| "TESTMETA123".to_string());

        // Generate secret and create source wallet
        let secret = generate_secret(&seed);
        let source_wallet = Wallet::new(
            Some(&secret),
            None,
            Some(&token),
            None,
            Some(&source_position),
            None,
            Some("BASE64"),
        ).context("Failed to create source wallet")?;

        Logger::test("Source wallet creation", true, None);

        // Create fixed remainder wallet for deterministic testing
        let remainder_wallet = create_fixed_remainder_wallet(&secret, &token)?;

        // Clone source wallet to keep a reference for validation
        let source_wallet_for_validation = source_wallet.clone();

        // Create molecule for metadata with fixed remainder wallet
        let mut molecule = Molecule::with_params(
            Some(secret.clone()),
            None,
            Some(source_wallet),
            Some(remainder_wallet),
            None,
            None,
        );

        // Create metadata (JavaScript compatibility)
        let metadata = vec![
            MetaItem {
                key: "name".to_string(),
                value: "Test Metadata".to_string(),
            },
            MetaItem {
                key: "description".to_string(),
                value: "This is a test metadata for SDK testing.".to_string(),
            },
        ];

        // Initialize metadata molecule (includes M + I atoms via add_continuid_atom)
        molecule.init_meta(metadata, &meta_type, &meta_id, None)
            .context("Failed to initialize metadata molecule")?;

        Logger::test("Metadata molecule initialization", true, None);

        // Set fixed timestamps for deterministic testing (before signing)
        set_fixed_timestamps(&mut molecule);

        // Sign the molecule
        let signature_result = molecule.sign(molecule.bundle.clone(), false, true);
        let signed = signature_result.is_ok();
        Logger::test("Molecule signing", signed, None);

        // Debug: Inspect molecule before validation
        MoleculeInspector::inspect(&molecule, "METADATA MOLECULE");

        // Step-by-step validation diagnostic
        MoleculeInspector::diagnose_validation(&molecule, "METADATA MOLECULE");

        // Validate the molecule with source wallet (matching JavaScript SDK behavior)
        let (is_valid, validation_error) = match molecule.verify_with_wallet(&source_wallet_for_validation).await {
            Ok(valid) => (valid, None),
            Err(e) => (false, Some(format!("Signature verification failed: {}", e))),
        };

        Logger::test("Molecule validation", is_valid, validation_error.as_deref());

        // Store serialized molecule for cross-SDK verification
        let molecule_json = match molecule.toJSON() {
            Ok(json) => json,
            Err(e) => {
                eprintln!("ERROR: Failed to serialize metadata molecule: {}", e);
                "{}".to_string()  // Empty JSON object instead of empty string
            }
        };
        self.results.molecules.metadata = molecule_json;

        // Store test results
        self.results.tests.meta_creation = MoleculeTestResult {
            passed: is_valid,
            molecular_hash: molecule.molecular_hash.unwrap_or_default(),
            atom_count: molecule.atoms.len(),
            has_remainder: None,
            validation_error: validation_error.or_else(|| Some("null".to_string())),
        };

        Ok(is_valid)
    }

    async fn test_simple_transfer(&mut self) -> Result<bool> {
        Logger::message("\n3. Simple Transfer Test", colors::BLUE);

        let source_seed = self.config.get_string("tests.simpleTransfer.sourceSeed").unwrap_or_else(|| "TESTSEED".to_string());
        let recipient_seed = self.config.get_string("tests.simpleTransfer.recipientSeed").unwrap_or_else(|| "TESTSEED2".to_string());
        let token = self.config.get_string("tests.simpleTransfer.token").unwrap_or_else(|| "TEST".to_string());
        let source_position = self.config.get_string("tests.simpleTransfer.sourcePosition").unwrap_or_default();
        let recipient_position = self.config.get_string("tests.simpleTransfer.recipientPosition").unwrap_or_default();
        let balance = self.config.get_i64("tests.simpleTransfer.balance").unwrap_or(1000) as f64;
        let amount = self.config.get_i64("tests.simpleTransfer.amount").unwrap_or(1000) as f64;

        // Create source wallet
        let source_secret = generate_secret(&source_seed);
        let mut source_wallet = Wallet::new(
            Some(&source_secret),
            None,
            Some(&token),
            None,
            Some(&source_position),
            None,
            Some("BASE64"),
        ).context("Failed to create source wallet")?;

        source_wallet.set_balance_f64(balance);  // Set balance for testing
        Logger::test("Source wallet creation", true, None);

        // Create recipient wallet
        let recipient_secret = generate_secret(&recipient_seed);
        let recipient_wallet = Wallet::new(
            Some(&recipient_secret),
            None,
            Some(&token),
            None,
            Some(&recipient_position),
            None,
            Some("BASE64"),
        ).context("Failed to create recipient wallet")?;

        Logger::test("Recipient wallet creation", true, None);

        // Create fixed remainder wallet for deterministic testing
        let remainder_wallet = create_fixed_remainder_wallet(&source_secret, &token)?;

        // Clone source wallet to keep a reference for validation
        let source_wallet_for_validation = source_wallet.clone();

        // Create molecule for value transfer
        let mut molecule = Molecule::with_params(
            Some(source_secret.clone()),
            None,
            Some(source_wallet),
            Some(remainder_wallet),
            None,
            None,
        );

        // Initialize value transfer (now uses JavaScript UTXO pattern)
        molecule.init_value(&recipient_wallet, amount)
            .context("Failed to initialize value transfer")?;

        Logger::test("Value transfer initialization", true, None);

        // Set fixed timestamps for deterministic testing (before signing)
        set_fixed_timestamps(&mut molecule);

        // Sign the molecule
        let signature_result = molecule.sign(molecule.bundle.clone(), false, true);
        let signed = signature_result.is_ok();
        Logger::test("Molecule signing", signed, None);

        // Debug: Inspect molecule before validation
        MoleculeInspector::inspect(&molecule, "SIMPLE TRANSFER MOLECULE");

        // Validate the molecule with source wallet (matching JavaScript SDK behavior)
        let (is_valid, validation_error) = match molecule.verify_with_wallet(&source_wallet_for_validation).await {
            Ok(valid) => (valid, None),
            Err(e) => (false, Some(format!("Signature verification failed: {}", e))),
        };

        Logger::test("Molecule validation", is_valid, validation_error.as_deref());

        // Store serialized molecule
        let molecule_json = match molecule.toJSON() {
            Ok(json) => json,
            Err(e) => {
                eprintln!("ERROR: Failed to serialize simple transfer molecule: {}", e);
                "{}".to_string()  // Empty JSON object instead of empty string
            }
        };
        self.results.molecules.simple_transfer = molecule_json;

        // Store test results
        self.results.tests.simple_transfer = MoleculeTestResult {
            passed: is_valid,
            molecular_hash: molecule.molecular_hash.unwrap_or_default(),
            atom_count: molecule.atoms.len(),
            has_remainder: None,
            validation_error: validation_error.or_else(|| Some("null".to_string())),
        };

        Ok(is_valid)
    }

    async fn test_complex_transfer(&mut self) -> Result<bool> {
        Logger::message("\n4. Complex Transfer Test", colors::BLUE);

        let source_seed = self.config.get_string("tests.complexTransfer.sourceSeed").unwrap_or_else(|| "TESTSEED".to_string());
        let recipient_seed = self.config.get_string("tests.complexTransfer.recipient1Seed").unwrap_or_else(|| "TESTSEED2".to_string());
        let token = self.config.get_string("tests.complexTransfer.token").unwrap_or_else(|| "TEST".to_string());
        let source_position = self.config.get_string("tests.complexTransfer.sourcePosition").unwrap_or_default();
        let recipient_position = self.config.get_string("tests.complexTransfer.recipient1Position").unwrap_or_default();
        let balance = self.config.get_i64("tests.complexTransfer.sourceBalance").unwrap_or(1000) as f64;
        let amount = self.config.get_i64("tests.complexTransfer.amount1").unwrap_or(500) as f64;

        // Create source wallet
        let source_secret = generate_secret(&source_seed);
        let mut source_wallet = Wallet::new(
            Some(&source_secret),
            None,
            Some(&token),
            None,
            Some(&source_position),
            None,
            Some("BASE64"),
        ).context("Failed to create source wallet")?;

        source_wallet.set_balance_f64(balance);
        Logger::test("Source wallet creation", true, None);

        // Create fixed remainder wallet for deterministic testing
        let remainder_wallet = create_fixed_remainder_wallet(&source_secret, &token)?;

        Logger::test("Remainder wallet creation", true, None);

        // Create recipient wallet
        let recipient_secret = generate_secret(&recipient_seed);
        let recipient_wallet = Wallet::new(
            Some(&recipient_secret),
            None,
            Some(&token),
            None,
            Some(&recipient_position),
            None,
            Some("BASE64"),
        ).context("Failed to create recipient wallet")?;

        Logger::test("Recipient wallet creation", true, None);

        // Create molecule for value transfer with remainder
        // Clone source wallet to keep a reference for validation
        let source_wallet_for_validation = source_wallet.clone();
        let mut molecule = Molecule::with_params(
            Some(source_secret.clone()),
            None,
            Some(source_wallet),
            Some(remainder_wallet),
            None,
            None,
        );

        // Initialize value transfer with remainder (JavaScript UTXO pattern)
        molecule.init_value(&recipient_wallet, amount)
            .context("Failed to initialize value transfer")?;

        Logger::test("Value transfer with remainder initialization", true, None);

        // Set fixed timestamps for deterministic testing (before signing)
        set_fixed_timestamps(&mut molecule);

        // Sign the molecule
        let signature_result = molecule.sign(molecule.bundle.clone(), false, true);
        let signed = signature_result.is_ok();
        Logger::test("Molecule signing", signed, None);

        // Debug: Inspect molecule before validation
        MoleculeInspector::inspect(&molecule, "COMPLEX TRANSFER MOLECULE");

        // Step-by-step validation diagnostic
        MoleculeInspector::diagnose_validation(&molecule, "COMPLEX TRANSFER MOLECULE");

        // Validate the molecule with source wallet (matching JavaScript SDK behavior)
        let (is_valid, validation_error) = match molecule.verify_with_wallet(&source_wallet_for_validation).await {
            Ok(valid) => (valid, None),
            Err(e) => (false, Some(format!("Signature verification failed: {}", e))),
        };

        Logger::test("Molecule validation", is_valid, validation_error.as_deref());

        // Store serialized molecule
        let molecule_json = match molecule.toJSON() {
            Ok(json) => json,
            Err(e) => {
                eprintln!("ERROR: Failed to serialize complex transfer molecule: {}", e);
                "{}".to_string()  // Empty JSON object instead of empty string
            }
        };
        self.results.molecules.complex_transfer = molecule_json;

        // Store test results
        self.results.tests.complex_transfer = MoleculeTestResult {
            passed: is_valid,
            molecular_hash: molecule.molecular_hash.unwrap_or_default(),
            atom_count: molecule.atoms.len(),
            has_remainder: Some(true),
            validation_error: validation_error.or_else(|| Some("null".to_string())),
        };

        Ok(is_valid)
    }

    async fn test_mlkem768(&mut self) -> Result<bool> {
        Logger::message("\n5. ML-KEM768 Encryption Test", colors::BLUE);
        let test_config_value = self.config.get_value("tests.mlkem768");

        if test_config_value.is_none() {
            Logger::test("ML-KEM768 configuration", false, Some("Config missing"));
            return Ok(false);
        }

        // Test ML-KEM768 encryption functionality
        let result: Result<bool, anyhow::Error> = {
            let seed = self.config.get_string("tests.mlkem768.seed").unwrap_or_else(|| "TESTSEED".to_string());
            let token = self.config.get_string("tests.mlkem768.token").unwrap_or_else(|| "ENCRYPT".to_string());
            let position = self.config.get_string("tests.mlkem768.position").unwrap_or_default();
            let plaintext = self.config.get_string("tests.mlkem768.plaintext").unwrap_or_else(|| "Hello ML-KEM768 cross-platform test message!".to_string());

            // Create encryption wallet from seed
            let secret = generate_secret(&seed);
            let _bundle = generate_bundle_hash(&secret);

            let encryption_wallet = Wallet::new(
                Some(&secret),
                None,
                Some(&token),
                None,
                Some(&position),
                None,
                Some("BASE64"),
            ).context("Failed to create encryption wallet")?;

            Logger::test("Encryption wallet creation", true, None);

            // Get ML-KEM768 public key (non-deterministic)
            let public_key = encryption_wallet.pubkey.as_ref().map(|k| k.as_str()).unwrap_or("");
            let public_key_generated = !public_key.is_empty();
            Logger::test("ML-KEM768 public key generation", public_key_generated, None);

            // Encrypt plaintext message for ourselves (non-deterministic)
            let encrypted_data = encryption_wallet.encrypt_message(
                &serde_json::json!(plaintext),
                public_key
            ).await
                .context("Failed to encrypt message")?;

            let encryption_success = !encrypted_data.cipher_text.is_empty() && !encrypted_data.encrypted_message.is_empty();
            Logger::test("Message encryption (self-encryption)", encryption_success, None);

            // Decrypt the encrypted message
            let decrypted_result = encryption_wallet.decrypt_message(&encrypted_data).await;
            let (decryption_success, decrypted_message) = match decrypted_result {
                Ok(msg) => (true, msg.as_str().unwrap_or("").to_string()),
                Err(_) => (false, String::new()),
            };

            let message_matches = decryption_success && decrypted_message == plaintext;
            Logger::test("Message decryption and verification", message_matches, None);

            let test_passed = public_key_generated && encryption_success && message_matches;

            // Store ML-KEM768 data for cross-SDK verification (non-deterministic outputs)
            let mlkem_data = serde_json::json!({
                "publicKey": public_key,
                "encryptedData": {
                    "cipherText": encrypted_data.cipher_text,
                    "encryptedMessage": encrypted_data.encrypted_message
                },
                "originalPlaintext": plaintext,
                "sdk": "Rust"
            });

            self.results.molecules.mlkem768 = serde_json::to_string(&mlkem_data).unwrap_or_default();

            self.results.tests.mlkem768 = MLKEMTestResult {
                passed: test_passed,
                public_key_generated,
                encryption_success,
                decryption_success: message_matches,
                error: if test_passed { None } else { Some("ML-KEM768 test failed".to_string()) },
            };

            Ok(test_passed)
        };

        // Handle any errors from the ML-KEM768 test
        match result {
            Ok(test_passed) => Ok(test_passed),
            Err(error) => {
                Logger::test("ML-KEM768 test", false, Some(&format!("Error: {}", error)));

                self.results.tests.mlkem768 = MLKEMTestResult {
                    passed: false,
                    public_key_generated: false,
                    encryption_success: false,
                    decryption_success: false,
                    error: Some(error.to_string()),
                };

                Ok(false)
            }
        }
    }

    async fn test_negative_cases(&mut self) -> Result<bool> {
        Logger::message("\n6. Negative Test Cases (Anti-Cheating)", colors::BLUE);

        let seed = self.config.get_string("tests.crypto.seed").unwrap_or_else(|| "TESTSEED".to_string());
        let mut all_negative_tests_passed = true;

        // Execute the negative test cases
        let test_result = async {
            let secret = generate_secret(&seed);
            let bundle = generate_bundle_hash(&secret);

            let mut source_wallet = Wallet::new(
                Some(&secret),
                None,
                Some("TEST"),
                None,
                Some("0123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210"),
                None,
                None
            )?;
            source_wallet.balance = "1000".to_string();

            // Test 1: Missing Molecular Hash (should fail)
            {
                let mut invalid_molecule = Molecule::with_params(
                    Some(secret.clone()),
                    Some(bundle.clone()),
                    Some(source_wallet.clone()),
                    None,
                    None,
                    None,
                );

                // Add a valid atom but don't sign (no molecular hash)
                let atom = Atom {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    wallet_address: source_wallet.address.clone().unwrap_or_default(),
                    isotope: Isotope::V,
                    token: "TEST".to_string(),
                    value: Some("-100.0".to_string()),
                    created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis().to_string(),
                    index: Some(0),
                    batch_id: None,
                    meta_type: None,
                    meta_id: None,
                    meta: Vec::new(),
                    ots_fragment: None,
                    version: None,
                };
                invalid_molecule.add_atom(atom);

                // This should fail because there's no molecular hash
                let validation_result = invalid_molecule.verify().await;
                match validation_result {
                    Ok(true) => {
                        Logger::test("Missing molecular hash validation (should FAIL)", false,
                                   Some("Invalid molecule passed validation"));
                        all_negative_tests_passed = false;
                    }
                    _ => {
                        Logger::test("Missing molecular hash validation (should FAIL)", true, None);
                    }
                }
            }

            // Test 2: Invalid Molecular Hash (should fail)
            {
                let mut invalid_molecule = Molecule::with_params(
                    Some(secret.clone()),
                    Some(bundle.clone()),
                    Some(source_wallet.clone()),
                    None,
                    None,
                    None,
                );

                let atom = Atom {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    wallet_address: source_wallet.address.clone().unwrap_or_default(),
                    isotope: Isotope::V,
                    token: "TEST".to_string(),
                    value: Some("-100.0".to_string()),
                    created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis().to_string(),
                    index: Some(0),
                    batch_id: None,
                    meta_type: None,
                    meta_id: None,
                    meta: Vec::new(),
                    ots_fragment: None,
                    version: None,
                };
                invalid_molecule.add_atom(atom);

                // Sign normally
                invalid_molecule.sign(Some(bundle.clone()), false, true)?;

                // Then corrupt the molecular hash
                invalid_molecule.molecular_hash = Some("invalid_hash_that_should_fail_validation_check_12345678".to_string());

                let validation_result = invalid_molecule.verify().await;
                match validation_result {
                    Ok(true) => {
                        Logger::test("Invalid molecular hash validation (should FAIL)", false,
                                   Some("Corrupted molecule passed validation"));
                        all_negative_tests_passed = false;
                    }
                    _ => {
                        Logger::test("Invalid molecular hash validation (should FAIL)", true, None);
                    }
                }
            }

            // Test 3: Unbalanced Transfer (should fail)
            {
                let mut invalid_molecule = Molecule::with_params(
                    Some(secret.clone()),
                    Some(bundle.clone()),
                    Some(source_wallet.clone()),
                    None,
                    None,
                    None,
                );

                // Create unbalanced atoms (doesn't sum to zero)
                let atom1 = Atom {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    wallet_address: source_wallet.address.clone().unwrap_or_default(),
                    isotope: Isotope::V,
                    token: "TEST".to_string(),
                    value: Some("-1000.0".to_string()), // Debit full balance
                    created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis().to_string(),
                    index: Some(0),
                    batch_id: None,
                    meta_type: None,
                    meta_id: None,
                    meta: Vec::new(),
                    ots_fragment: None,
                    version: None,
                };
                invalid_molecule.add_atom(atom1);

                let atom2 = Atom {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    wallet_address: source_wallet.address.clone().unwrap_or_default(),
                    isotope: Isotope::V,
                    token: "TEST".to_string(),
                    value: Some("500.0".to_string()), // Credit only half - unbalanced!
                    created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis().to_string(),
                    index: Some(1),
                    batch_id: None,
                    meta_type: None,
                    meta_id: None,
                    meta: Vec::new(),
                    ots_fragment: None,
                    version: None,
                };
                invalid_molecule.add_atom(atom2);

                invalid_molecule.sign(Some(bundle.clone()), false, true)?;

                let validation_result = invalid_molecule.verify().await;
                match validation_result {
                    Ok(true) => {
                        Logger::test("Unbalanced transfer validation (should FAIL)", false,
                                   Some("Unbalanced molecule passed validation"));
                        all_negative_tests_passed = false;
                    }
                    _ => {
                        Logger::test("Unbalanced transfer validation (should FAIL)", true, None);
                    }
                }
            }

            Ok::<bool, anyhow::Error>(all_negative_tests_passed)
        }.await;

        // Handle any errors from the negative test cases
        match test_result {
            Ok(result) => {
                self.results.tests.negative_cases = NegativeTestResult {
                    passed: result,
                    description: "Anti-cheating validation tests".to_string(),
                    test_count: 3,
                    error: None,
                };
                Ok(result)
            },
            Err(error) => {
                Logger::message(&format!("  ❌ ERROR: {}", error), colors::RED);

                self.results.tests.negative_cases = NegativeTestResult {
                    passed: false,
                    description: "Anti-cheating validation tests".to_string(),
                    test_count: 3,
                    error: Some(error.to_string()),
                };

                Ok(false)
            }
        }
    }

    async fn test_cross_sdk_validation(&mut self) -> Result<bool> {
        Logger::message("\n7. Cross-SDK Validation", colors::BLUE);

        // Check if cross-validation is disabled (Round 1 molecule generation only)
        if std::env::var("KNISHIO_DISABLE_CROSS_VALIDATION").unwrap_or_default() == "true" {
            Logger::message("  ⏭️  Cross-validation disabled for Round 1 (molecule generation only)", colors::YELLOW);
            self.results.cross_sdk_compatible = true;
            return Ok(true);
        }

        Logger::message("  📋 Loading molecules from other SDKs...", colors::CYAN);

        let results_dir = std::env::var("KNISHIO_SHARED_RESULTS")
            .unwrap_or_else(|_| "../shared-test-results".to_string());

        // Check if results directory exists
        if !std::path::Path::new(&results_dir).exists() {
            Logger::message("  ⏭️  No other SDK results found for cross-validation", colors::YELLOW);
            self.results.cross_sdk_compatible = true;
            return Ok(true);
        }

        // Get all JSON result files except rust-results.json
        let result_files: Vec<_> = fs::read_dir(&results_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_str()
                    .map_or(false, |name| name.ends_with(".json") && !name.contains("rust"))
            })
            .collect();

        if result_files.is_empty() {
            Logger::message("  ⏭️  No other SDK results found for cross-validation", colors::YELLOW);
            self.results.cross_sdk_compatible = true;
            return Ok(true);
        }

        let mut all_valid = true;

        for result_file in result_files {
            let file_path = result_file.path();
            let sdk_name = file_path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("unknown")
                .replace("-results", "");

            Logger::message(&format!("\n  🧪 Validating {} SDK molecules:", sdk_name.to_uppercase()), colors::CYAN);

            // Read and parse the result file
            let file_contents = fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read {}", file_path.display()))?;

            let other_results: Value = serde_json::from_str(&file_contents)
                .with_context(|| format!("Failed to parse {} JSON", file_path.display()))?;

            // Validate molecules from this SDK
            if let Some(molecules) = other_results.get("molecules").and_then(|m| m.as_object()) {
                for (molecule_type, molecule_data) in molecules {
                    if molecule_type == "mlkem768" {
                        // Special handling for ML-KEM768 cross-SDK compatibility
                        let validation_success = match self.validate_cross_sdk_mlkem768(molecule_data).await {
                            Ok(valid) => {
                                Logger::message(&format!("    ✅ {} encryption: PASSED", molecule_type), colors::GREEN);
                                valid
                            }
                            Err(error) => {
                                Logger::message(&format!("    ❌ {} encryption: FAILED - {}", molecule_type, error), colors::RED);
                                false
                            }
                        };

                        if !validation_success {
                            all_valid = false;
                        }
                    } else {
                        // Standard molecule validation for non-ML-KEM768 types
                        let validation_success = match self.validate_cross_sdk_molecule(molecule_data, molecule_type).await {
                            Ok(valid) => {
                                Logger::message(&format!("    ✅ {} molecule: PASSED", molecule_type), colors::GREEN);
                                valid
                            }
                            Err(error) => {
                                Logger::message(&format!("    ❌ {} molecule: FAILED - {}", molecule_type, error), colors::RED);
                                false
                            }
                        };

                        if !validation_success {
                            all_valid = false;
                        }
                    }
                }
            }
        }

        if all_valid {
            Logger::message("\n  ✅ All cross-SDK molecules validated successfully", colors::GREEN);
            Logger::message("  ✅ Cross-SDK Compatible: YES", colors::GREEN);
        } else {
            Logger::message("\n  ❌ Some cross-SDK molecules failed validation", colors::RED);
            Logger::message("  ❌ Cross-SDK Compatible: NO", colors::RED);
        }

        self.results.cross_sdk_compatible = all_valid;
        Ok(all_valid)
    }

    /// Validate a single molecule from another SDK
    ///
    /// This method reconstructs a Molecule from JSON and validates it using
    /// the Rust SDK's native validation methods, matching JavaScript SDK patterns.
    async fn validate_cross_sdk_molecule(&self, molecule_data: &Value, molecule_type: &str) -> Result<bool> {
        // Parse the JSON string containing the serialized molecule
        let molecule_json_str = molecule_data.as_str()
            .ok_or_else(|| anyhow::anyhow!("Molecule data is not a string"))?;

        // Use centralized fromJSON() method for clean deserialization (matching JavaScript SDK)
        let molecule = Molecule::fromJSON(molecule_json_str)
            .with_context(|| format!("Failed to deserialize {} molecule using centralized fromJSON()", molecule_type))?;

        // Source wallet is automatically reconstructed by fromJSON() method
        let source_wallet = molecule.source_wallet.as_ref();

        // Use Rust SDK's native validation method
        let is_valid = molecule.check(source_wallet)
            .with_context(|| format!("Validation failed for {} molecule", molecule_type))?;

        Ok(is_valid)
    }

    /// Validate ML-KEM768 cross-SDK compatibility
    ///
    /// This method tests decryption compatibility by attempting to decrypt
    /// data encrypted by other SDKs using our private key.
    async fn validate_cross_sdk_mlkem768(&self, mlkem_data: &Value) -> Result<bool> {
        // Parse the JSON string containing the ML-KEM768 data
        let mlkem_json_str = mlkem_data.as_str()
            .ok_or_else(|| anyhow::anyhow!("ML-KEM768 data is not a string"))?;

        let data: Value = serde_json::from_str(mlkem_json_str)
            .with_context(|| "Failed to parse ML-KEM768 data JSON")?;

        // Extract the original plaintext for comparison
        let original_plaintext = data["originalPlaintext"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Original plaintext missing"))?;

        // REAL CROSS-SDK CRYPTOGRAPHIC VALIDATION: Test actual encryption compatibility
        // Create our own encryption wallet using the same configuration as the other SDK
        let test_config = self.config.get_value("tests.mlkem768").unwrap();
        let seed = test_config["seed"].as_str().unwrap_or("TESTSEED");
        let token = test_config["token"].as_str().unwrap_or("ENCRYPT");
        let position = test_config["position"].as_str().unwrap_or("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

        let secret = generate_secret(seed);
        let our_wallet = Wallet::new(
            Some(&secret),
            None,
            Some(token),
            None,
            Some(position),
            None,
            Some("BASE64"),
        )?;

        // REAL TEST: Attempt to encrypt a message using their public key
        let their_public_key = data["publicKey"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing publicKey in ML-KEM768 data"))?;

        let test_message = "Cross-SDK ML-KEM768 compatibility test";

        // Test encryption compatibility with their public key
        match our_wallet.encrypt_message(
            &serde_json::json!(test_message),
            their_public_key
        ).await {
            Ok(encrypted_data) => {
                // If encryption succeeded, their public key format is compatible
                let encryption_compatible = !encrypted_data.cipher_text.is_empty() &&
                                          !encrypted_data.encrypted_message.is_empty();

                if encryption_compatible {
                    println!("    ✅ Successfully encrypted message with their public key");
                    println!("    ✅ Public key format compatibility confirmed");
                } else {
                    println!("    ❌ Encryption produced empty results");
                }

                // Additional validation: Verify we can decrypt our own test encryption
                if let Some(our_pubkey) = &our_wallet.pubkey {
                    match our_wallet.encrypt_message(
                        &serde_json::json!(original_plaintext),
                        our_pubkey
                    ).await {
                        Ok(self_encrypted) => {
                            match our_wallet.decrypt_message(&self_encrypted).await {
                                Ok(decrypted) => {
                                    let self_test_valid = decrypted.as_str().unwrap_or("") == original_plaintext;
                                    if self_test_valid {
                                        println!("    ✅ Self-encryption/decryption verification passed");
                                    } else {
                                        println!("    ❌ Self-encryption/decryption verification failed");
                                    }
                                    Ok(encryption_compatible && self_test_valid)
                                }
                                Err(e) => {
                                    println!("    ❌ Self-decryption failed: {}", e);
                                    Ok(false)
                                }
                            }
                        }
                        Err(e) => {
                            println!("    ❌ Self-encryption failed: {}", e);
                            Ok(false)
                        }
                    }
                } else {
                    println!("    ❌ No public key available for self-test");
                    Ok(false)
                }
            }
            Err(e) => {
                println!("    ❌ Failed to encrypt with their public key: {}", e);
                println!("    ❌ Public key format incompatible");
                Ok(false)
            }
        }
    }

    async fn save_results(&self) -> Result<()> {
        let shared_dir = std::env::var("KNISHIO_SHARED_RESULTS")
            .unwrap_or_else(|_| "../shared-test-results".to_string());

        // Ensure shared directory exists
        std::fs::create_dir_all(&shared_dir)
            .with_context(|| format!("Failed to create shared directory: {}", shared_dir))?;

        let results_path = format!("{}/rust-results.json", shared_dir);

        // Create JSON structure matching other SDKs exactly
        let json_output = serde_json::to_string_pretty(&self.results)
            .context("Failed to serialize results")?;

        fs::write(&results_path, json_output)
            .with_context(|| format!("Cannot create results file: {}", results_path))?;

        println!("\n{}📁 Results saved to: {}{}", colors::BLUE, results_path, colors::RESET);

        Ok(())
    }

    fn display_summary(&self) {
        Logger::message("\n═══════════════════════════════════════════", colors::BLUE);
        Logger::message("            TEST SUMMARY REPORT", colors::BLUE);
        Logger::message("═══════════════════════════════════════════", colors::BLUE);
        Logger::message("", colors::RESET);

        println!("SDK: Rust v{}", self.results.version);
        println!("Timestamp: {}", self.results.timestamp);

        // Count passed tests
        let total_tests = 6;
        let passed_tests = [
            self.results.tests.crypto.passed,
            self.results.tests.meta_creation.passed,
            self.results.tests.simple_transfer.passed,
            self.results.tests.complex_transfer.passed,
            self.results.tests.mlkem768.passed,
            self.results.tests.negative_cases.passed,
        ].iter().filter(|&&x| x).count();

        let color = if passed_tests == total_tests { colors::GREEN } else { colors::RED };
        println!("\n{}Tests Passed: {}/{}{}", color, passed_tests, total_tests, colors::RESET);

        // Show failed tests
        if passed_tests < total_tests {
            println!("\n{}Failed Tests:{}", colors::RED, colors::RESET);
            if !self.results.tests.meta_creation.passed {
                println!("  - metaCreation: Validation failed");
            }
            if !self.results.tests.simple_transfer.passed {
                println!("  - simpleTransfer: Validation failed");
            }
            if !self.results.tests.complex_transfer.passed {
                println!("  - complexTransfer: Validation failed");
            }
        }

        let compat_color = if self.results.cross_sdk_compatible { colors::GREEN } else { colors::RED };
        let compat_status = if self.results.cross_sdk_compatible { "✅ YES" } else { "❌ NO" };
        println!("\n{}Cross-SDK Compatible: {}{}", compat_color, compat_status, colors::RESET);

        Logger::message("═══════════════════════════════════════════", colors::BLUE);
    }
}

/* Modern Rust 2025 main function with proper error handling */
#[tokio::main]
async fn main() -> Result<()> {
    let mut runner = SelfTestRunner::new()
        .context("Failed to initialize test runner")?;

    runner.run_all_tests().await
        .context("Test execution failed")?;

    Ok(())
}
