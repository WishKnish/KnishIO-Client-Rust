//! Molecule validation and integrity checking
//!
//! This module contains the CheckMolecule implementation that provides comprehensive
//! validation of molecular transactions, ensuring exact compatibility with the
//! JavaScript SDK's CheckMolecule.js class.

use crate::atom::Atom;
use crate::molecule::Molecule;
use crate::wallet::Wallet;
use crate::types::Isotope;
use crate::error::{KnishIOError, Result};
use crate::meta::Meta;
use crate::crypto::shake256;
use crate::rules::Rule;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;

/// Result of verifying a single molecule's integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeIntegrityResult {
    /// Molecular hash of the verified molecule
    pub molecular_hash: Option<String>,
    /// Whether the molecule passed integrity verification
    pub verified: bool,
    /// Error message if verification failed
    pub error: Option<String>,
}

/// Result of verifying integrity across all molecules in a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    /// Whether all molecules passed verification
    pub verified: bool,
    /// Individual molecule verification results
    pub molecules: Vec<MoleculeIntegrityResult>,
}

/// Comprehensive molecule validation class
///
/// Equivalent to CheckMolecule.js, this class provides thorough validation
/// of molecular transactions including signature verification, balance checking,
/// isotope validation, and policy compliance.
#[derive(Debug)]
pub struct CheckMolecule<'a> {
    molecule: &'a Molecule,
}

impl<'a> CheckMolecule<'a> {
    /// Create a new CheckMolecule validator
    ///
    /// # Arguments
    ///
    /// * `molecule` - Molecule to validate
    ///
    /// # Returns
    ///
    /// CheckMolecule instance ready for validation
    ///
    /// # Errors
    ///
    /// Returns error if molecule is missing required components
    pub fn new(molecule: &'a Molecule) -> Result<Self> {
        // No molecular hash?
        if molecule.molecular_hash.is_none() {
            return Err(KnishIOError::MolecularHashMissing);
        }

        // No atoms?
        if molecule.atoms.is_empty() {
            return Err(KnishIOError::AtomsMissing);
        }

        // Check atom indexes
        for atom in &molecule.atoms {
            if atom.index.is_none() {
                return Err(KnishIOError::AtomIndex);
            }
        }

        Ok(CheckMolecule { molecule })
    }

    /// Comprehensive verification of the molecule
    ///
    /// Runs all validation checks in sequence, matching the JavaScript implementation.
    ///
    /// # Arguments
    ///
    /// * `sender_wallet` - Optional sender wallet for balance validation
    ///
    /// # Returns
    ///
    /// True if all validations pass, error otherwise
    pub fn verify(&self, sender_wallet: Option<&Wallet>) -> Result<bool> {
        // Run all validation checks in order (matching JS CheckMolecule.verify)
        self.molecular_hash()?;
        self.ots()?;
        self.batch_id()?;
        self.continu_id()?;
        self.isotope_m()?;
        self.isotope_t()?;
        self.isotope_c()?;
        self.isotope_u()?;
        self.isotope_i()?;
        self.isotope_r()?;
        self.isotope_p()?;
        self.isotope_a()?;
        self.isotope_b()?;
        self.isotope_f()?;
        self.isotope_v(sender_wallet)?;

        Ok(true)
    }

    /// Validate ContinuID requirements
    ///
    /// Equivalent to CheckMolecule.continuId() in JavaScript
    fn continu_id(&self) -> Result<bool> {
        let first_atom = &self.molecule.atoms[0];

        if first_atom.token == "USER" && self.get_isotopes(&[Isotope::I]).is_empty() {
            return Err(KnishIOError::AtomsMissing);
        }

        Ok(true)
    }

    /// Validate batch ID consistency
    ///
    /// Equivalent to CheckMolecule.batchId() in JavaScript
    fn batch_id(&self) -> Result<bool> {
        if !self.molecule.atoms.is_empty() {
            let signing_atom = &self.molecule.atoms[0];

            if signing_atom.isotope == Isotope::V && signing_atom.batch_id.is_some() {
                let atoms = self.get_isotopes(&[Isotope::V]);
                let remainder_atom = &atoms[atoms.len() - 1];

                if signing_atom.batch_id != remainder_atom.batch_id {
                    return Err(KnishIOError::BatchId);
                }

                for atom in &atoms {
                    if atom.batch_id.is_none() {
                        return Err(KnishIOError::BatchId);
                    }
                }
            }

            return Ok(true);
        }

        Err(KnishIOError::BatchId)
    }

    /// Validate Identity isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeI() in JavaScript
    fn isotope_i(&self) -> Result<bool> {
        for atom in self.get_isotopes(&[Isotope::I]) {
            if atom.token != "USER" {
                return Err(KnishIOError::WrongTokenType);
            }

            if atom.index == Some(0) {
                return Err(KnishIOError::AtomIndex);
            }
        }

        Ok(true)
    }

    /// Validate Authorization isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeU() in JavaScript
    fn isotope_u(&self) -> Result<bool> {
        for atom in self.get_isotopes(&[Isotope::U]) {
            if atom.token != "AUTH" {
                return Err(KnishIOError::WrongTokenType);
            }

            if atom.index != Some(0) {
                return Err(KnishIOError::AtomIndex);
            }
        }

        Ok(true)
    }

    /// Validate Metadata isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeM() in JavaScript
    fn isotope_m(&self) -> Result<bool> {
        let policy_array = ["readPolicy", "writePolicy"];

        for atom in self.get_isotopes(&[Isotope::M]) {
            if atom.meta.is_empty() {
                return Err(KnishIOError::MetaMissing);
            }

            if atom.token != "USER" {
                return Err(KnishIOError::WrongTokenType);
            }

            let metas = Meta::aggregate_meta(&atom.meta);

            for key in &policy_array {
                if let Some(policy_json) = metas.get(*key) {
                    let policy: HashMap<String, serde_json::Value> = 
                        serde_json::from_str(policy_json)
                            .map_err(|_| KnishIOError::PolicyInvalid)?;

                    for (policy_name, policy_value) in policy {
                        if !policy_array.contains(&policy_name.as_str()) {
                            if !metas.contains_key(&policy_name) {
                                return Err(KnishIOError::PolicyInvalid);
                            }

                            if let Some(values) = policy_value.as_array() {
                                for value in values {
                                    if let Some(val_str) = value.as_str() {
                                        if !Wallet::is_bundle_hash(val_str) && 
                                           !["all", "self"].contains(&val_str) {
                                            return Err(KnishIOError::PolicyInvalid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// Validate Creation isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeC() in JavaScript
    fn isotope_c(&self) -> Result<bool> {
        for atom in self.get_isotopes(&[Isotope::C]) {
            if atom.token != "USER" {
                return Err(KnishIOError::WrongTokenType);
            }

            if atom.index != Some(0) {
                return Err(KnishIOError::AtomIndex);
            }
        }

        Ok(true)
    }

    /// Validate Token isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeT() in JavaScript
    fn isotope_t(&self) -> Result<bool> {
        for atom in self.get_isotopes(&[Isotope::T]) {
            let meta = atom.aggregated_meta();
            let meta_type = atom.meta_type.as_deref().unwrap_or("").to_lowercase();

            if meta_type == "wallet" {
                for key in &["position", "bundle"] {
                    if !meta.contains_key(*key) || meta.get(*key).unwrap_or(&String::new()).is_empty() {
                        return Err(KnishIOError::MetaMissing);
                    }
                }
            }

            for key in &["token"] {
                if !meta.contains_key(*key) || meta.get(*key).unwrap_or(&String::new()).is_empty() {
                    return Err(KnishIOError::MetaMissing);
                }
            }

            if atom.token != "USER" {
                return Err(KnishIOError::WrongTokenType);
            }

            if atom.index != Some(0) {
                return Err(KnishIOError::AtomIndex);
            }
        }

        Ok(true)
    }

    /// Validate Rule isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeR() in JavaScript
    fn isotope_r(&self) -> Result<bool> {
        for atom in self.get_isotopes(&[Isotope::R]) {
            let metas = atom.aggregated_meta();

            if let Some(policy_json) = metas.get("policy") {
                let policy: HashMap<String, serde_json::Value> = 
                    serde_json::from_str(policy_json)
                        .map_err(|_| KnishIOError::MetaMissing)?;

                for key in policy.keys() {
                    if !["read", "write"].contains(&key.as_str()) {
                        return Err(KnishIOError::MetaMissing);
                    }
                }
            }

            if let Some(rule_json) = metas.get("rule") {
                let rules: serde_json::Value = 
                    serde_json::from_str(rule_json)
                        .map_err(|_| KnishIOError::MetaMissing)?;

                if !rules.is_array() {
                    return Err(KnishIOError::MetaMissing);
                }

                let rules_array = rules.as_array().unwrap();
                
                if rules_array.is_empty() {
                    return Err(KnishIOError::MetaMissing);
                }

                // Validate individual rules using Rule::from_object (equivalent to Rule.toObject in JS)
                for rule_data in rules_array {
                    // Validate that each rule can be properly parsed using Rule::from_object
                    Rule::from_object(rule_data)
                        .map_err(|_| KnishIOError::MetaMissing)?;
                }
            }
        }

        Ok(true)
    }

    /// Validate Peering isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeP() in JavaScript
    fn isotope_p(&self) -> Result<bool> {
        for atom in self.get_isotopes(&[Isotope::P]) {
            if atom.token != "USER" {
                return Err(KnishIOError::WrongTokenType);
            }

            let metas = Meta::aggregate_meta(&atom.meta);

            match metas.get("peerHost") {
                Some(host) if !host.is_empty() => {}
                _ => return Err(KnishIOError::MetaMissing),
            }
        }

        Ok(true)
    }

    /// Validate Append-request isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeA() in JavaScript
    fn isotope_a(&self) -> Result<bool> {
        for atom in self.get_isotopes(&[Isotope::A]) {
            if atom.token != "USER" {
                return Err(KnishIOError::WrongTokenType);
            }

            if atom.meta_type.as_deref().unwrap_or("").is_empty() {
                return Err(KnishIOError::MetaMissing);
            }

            if atom.meta_id.as_deref().unwrap_or("").is_empty() {
                return Err(KnishIOError::MetaMissing);
            }

            let metas = Meta::aggregate_meta(&atom.meta);

            match metas.get("action") {
                Some(action) if !action.is_empty() => {}
                _ => return Err(KnishIOError::MetaMissing),
            }
        }

        Ok(true)
    }

    /// Validate Buffer/Exchange isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeB() in JavaScript.
    ///
    /// Buffer molecules are cross-isotope: their V atoms do not balance on their own
    /// because a B atom absorbs the difference. `isotope_v` therefore skips its V-only
    /// conservation checks whenever B (or F) atoms are present, and conservation is
    /// enforced here instead, over the combined V+B set.
    fn isotope_b(&self) -> Result<bool> {
        let isotope_b = self.get_isotopes(&[Isotope::B]);

        if isotope_b.is_empty() {
            return Ok(true);
        }

        for atom in &isotope_b {
            // B atoms must reference a wallet bundle
            if atom.meta_type.as_deref() != Some("walletBundle") {
                return Err(KnishIOError::MetaMissing);
            }

            if atom.meta_id.as_deref().unwrap_or("").is_empty() {
                return Err(KnishIOError::MetaMissing);
            }

            // Value must be parseable as a number
            let value: f64 = match atom.value.as_ref().map(|v| v.parse::<f64>()) {
                Some(Ok(parsed)) if !parsed.is_nan() => parsed,
                _ => return Err(KnishIOError::TransferMalformed),
            };
            let _ = value;
        }

        // V+B balance conservation: sum of all V and B atom values must equal zero
        let v_atoms = self.get_isotopes(&[Isotope::V]);
        if !v_atoms.is_empty() {
            let sum: f64 = v_atoms.iter().chain(isotope_b.iter())
                .filter_map(|a| a.value.as_ref().and_then(|v| v.parse::<f64>().ok()))
                .filter(|v| !v.is_nan())
                .sum();

            if sum != 0.0 {
                return Err(KnishIOError::TransferUnbalanced);
            }
        }

        Ok(true)
    }

    /// Validate Fusion/NFT isotope atoms
    ///
    /// Equivalent to CheckMolecule.isotopeF() in JavaScript.
    ///
    /// Mirrors `isotope_b` and additionally forbids negative values. Must stay paired with
    /// the `has_cross_isotope` gate in `isotope_v`, which is keyed on B *or* F: without
    /// this check an F-isotope molecule would skip V-only conservation with nothing
    /// validating V+F conservation in its place.
    fn isotope_f(&self) -> Result<bool> {
        let isotope_f = self.get_isotopes(&[Isotope::F]);

        if isotope_f.is_empty() {
            return Ok(true);
        }

        for atom in &isotope_f {
            // F atoms must reference a wallet bundle
            if atom.meta_type.as_deref() != Some("walletBundle") {
                return Err(KnishIOError::MetaMissing);
            }

            if atom.meta_id.as_deref().unwrap_or("").is_empty() {
                return Err(KnishIOError::MetaMissing);
            }

            let value: f64 = match atom.value.as_ref().map(|v| v.parse::<f64>()) {
                Some(Ok(parsed)) if !parsed.is_nan() => parsed,
                _ => return Err(KnishIOError::TransferMalformed),
            };

            if value < 0.0 {
                return Err(KnishIOError::TransferMalformed);
            }
        }

        // V+F balance conservation: sum of all V and F atom values must equal zero
        let v_atoms = self.get_isotopes(&[Isotope::V]);
        if !v_atoms.is_empty() {
            let sum: f64 = v_atoms.iter().chain(isotope_f.iter())
                .filter_map(|a| a.value.as_ref().and_then(|v| v.parse::<f64>().ok()))
                .filter(|v| !v.is_nan())
                .sum();

            if sum != 0.0 {
                return Err(KnishIOError::TransferUnbalanced);
            }
        }

        Ok(true)
    }

    /// Validate Value isotope atoms (transfer validation)
    ///
    /// Equivalent to CheckMolecule.isotopeV() in JavaScript
    fn isotope_v(&self, sender_wallet: Option<&Wallet>) -> Result<bool> {
        let isotope_v = self.get_isotopes(&[Isotope::V]);

        if isotope_v.is_empty() {
            return Ok(true);
        }

        // B/F isotope molecules have V-atoms that don't sum to zero on their own (the B/F
        // atom absorbs the difference), so the plain V-conservation check is skipped when a
        // cross-isotope is present — mirroring JS CheckMolecule's `!hasCrossIsotope` gate.
        let has_cross_isotope = !self.get_isotopes(&[Isotope::B, Isotope::F]).is_empty();

        let first_atom = &self.molecule.atoms[0];

        // Handle simple 2-atom transfer case (plain V-debit + V-remainder).
        //
        // Gated on !has_cross_isotope to mirror JS CheckMolecule.js:507. A buffer deposit
        // is [V, B, V] — two V atoms with a V first atom — so it would otherwise enter this
        // branch and return early. That happened to yield the right answer, but by a
        // shape coincidence rather than by the cross-isotope rule; a withdraw ([B, V, B])
        // has the same conservation semantics and does not match this shape at all.
        // Skipping the branch entirely for B/F molecules makes both directions take the
        // same path, which is what makes the behaviour isotope-driven instead of
        // shape-driven.
        if !has_cross_isotope && first_atom.isotope == Isotope::V && isotope_v.len() == 2 {
            let end_atom = &isotope_v[isotope_v.len() - 1];

            if first_atom.token != end_atom.token {
                return Err(KnishIOError::TransferMismatched);
            }

            let end_value: f64 = end_atom.value.as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);

            if end_value < 0.0 {
                return Err(KnishIOError::TransferMalformed);
            }

            // A plain 2-atom V transfer (no B/F isotope to absorb the difference) must
            // balance to zero, mirroring JS CheckMolecule.isotopeV's firstAtom+endAtom sum.
            if !has_cross_isotope {
                let first_value: f64 = first_atom.value.as_ref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                if first_value + end_value != 0.0 {
                    return Err(KnishIOError::TransferUnbalanced);
                }
            }

            return Ok(true);
        }

        let mut sum = 0.0;
        let mut value: f64 = 0.0;

        for (index, atom) in self.molecule.atoms.iter().enumerate() {
            // Not V? Next...
            if atom.isotope != Isotope::V {
                continue;
            }

            // Making sure we're in number land
            value = atom.value.as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);

            if value.is_nan() {
                return Err(KnishIOError::Custom("Invalid isotope V values".to_string()));
            }

            // Making sure all V atoms of the same token
            if atom.token != first_atom.token {
                return Err(KnishIOError::TransferMismatched);
            }

            // Checking non-primary atoms
            if index > 0 {
                // Negative V atom in a non-primary position?
                if value < 0.0 {
                    return Err(KnishIOError::TransferMalformed);
                }

                // Cannot be sending and receiving from the same address
                if atom.wallet_address == first_atom.wallet_address {
                    return Err(KnishIOError::TransferToSelf);
                }
            }

            // Adding this Atom's value to the total sum
            sum += value;
        }

        // V-only conservation: all V atoms must sum to zero (skip for B/F cross-isotope,
        // where the balancing atom is a different isotope and conservation is enforced by
        // isotope_b()/isotope_f() instead).
        //
        // This gate was missing, which is what rejected buffer WITHDRAWALS: a withdraw is
        // [B, V, B], so its first atom is B, it never enters the two-V branch above, and it
        // fell through to here where a lone V atom cannot sum to zero. Deposits ([V, B, V])
        // entered that branch and returned early, so only one direction ever failed.
        if !has_cross_isotope && sum != 0.0 {
            return Err(KnishIOError::TransferUnbalanced);
        }

        // If we're provided with a senderWallet argument, we can perform additional checks
        if let Some(sender) = sender_wallet {
            value = first_atom.value.as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);

            if value.is_nan() {
                return Err(KnishIOError::Custom("Invalid isotope V values".to_string()));
            }

            let remainder = sender.balance_as_i128() as f64 + value;

            // Is there enough balance to send?
            if remainder < 0.0 {
                return Err(KnishIOError::TransferBalance);
            }

            // Does the remainder match what should be there in the source wallet, if provided?
            // Skip for cross-isotope (B/F) — conservation is validated by isotope_b()/isotope_f()
            if !has_cross_isotope && remainder != sum {
                return Err(KnishIOError::TransferRemainder);
            }
        } else if value != 0.0 {
            // No senderWallet, but have a remainder?
            return Err(KnishIOError::TransferRemainder);
        }

        Ok(true)
    }

    /// Verify molecular hash integrity
    ///
    /// Equivalent to CheckMolecule.molecularHash() in JavaScript
    fn molecular_hash(&self) -> Result<bool> {
        let computed_hash = Atom::hash_atoms(&self.molecule.atoms, "base17")?;
        
        if let Some(ref stored_hash) = self.molecule.molecular_hash {
            if stored_hash != &computed_hash {
                return Err(KnishIOError::MolecularHashMismatch);
            }
        }

        Ok(true)
    }

    /// Verify one-time signature (OTS)
    ///
    /// Equivalent to CheckMolecule.ots() in JavaScript
    pub fn ots(&self) -> Result<bool> {
        // Convert Hm to numeric notation via EnumerateMolecule(Hm)
        let normalized_hash = self.molecule.normalized_hash()?;

        // Rebuilding OTS out of all the atoms
        let mut ots = String::new();
        for atom in &self.molecule.atoms {
            if let Some(ref fragment) = atom.ots_fragment {
                ots.push_str(fragment);
            }
        }

        // Wrong size? Maybe it's compressed
        if ots.len() != 2048 {
            // Attempting decompression
            ots = Self::base64_to_hex(&ots)?;

            // Still wrong? That's a failure
            if ots.len() != 2048 {
                return Err(KnishIOError::SignatureMalformed);
            }
        }

        // Subdivide Kk into 16 segments of 256 bytes (128 characters) each
        let ots_chunks = Self::chunk_substr(&ots, 128);

        let mut key_fragments = String::new();

        for (index, chunk) in ots_chunks.iter().enumerate() {
            let mut working_chunk = chunk.clone();

            // WOTS+ verification: condition should be 8 + normalized_hash[index]
            // This is opposite of signing which uses (8 - normalizedHash[index])
            // normalized_hash[index] is -8 to 8, so condition is 0 to 16
            let condition = (8 + normalized_hash[index] as i32) as usize;
            for _ in 0..condition {
                working_chunk = shake256(&working_chunk, 512);
            }

            key_fragments.push_str(&working_chunk);
        }

        // The reconstructed key_fragments is now the original signing key
        // JavaScript doesn't use generate_address here - it uses a simpler process:
        // 1. Create digest from key_fragments (8192 bits)
        // 2. Create address from digest (256 bits)
        
        // Absorb the hashed Kk into the sponge to receive the digest Dk
        let digest = shake256(&key_fragments, 8192);
        
        // Squeeze the sponge to retrieve a 128 byte (64 character) string that should match the sender's wallet address
        let address = shake256(&digest, 256);

        // Signing atom
        let signing_atom = &self.molecule.atoms[0];

        // Get a signing address
        let mut signing_address = signing_atom.wallet_address.clone();

        // Get signing wallet from first atom's metas
        let meta_map = signing_atom.aggregated_meta();
        let signing_wallet = meta_map.get("signingWallet");

        // Try to get custom signing address from the metas (local molecule with server secret)
        if let Some(signing_wallet_json) = signing_wallet {
            if let Ok(wallet_data) = serde_json::from_str::<HashMap<String, serde_json::Value>>(signing_wallet_json) {
                if let Some(addr) = wallet_data.get("address").and_then(|v| v.as_str()) {
                    signing_address = addr.to_string();
                }
            }
        }

        // JavaScript compares hex addresses directly
        // The signing_address from wallet is already in hex format
        // No conversion needed - both are hex
        
        if address != signing_address {
            return Err(KnishIOError::SignatureMismatch);
        }

        Ok(true)
    }

    /// Helper method to get atoms by isotope type(s)
    fn get_isotopes(&self, isotopes: &[Isotope]) -> Vec<&Atom> {
        self.molecule.atoms
            .iter()
            .filter(|atom| isotopes.contains(&atom.isotope))
            .collect()
    }

    /// Convert base64 to hexadecimal string
    fn base64_to_hex(base64_str: &str) -> Result<String> {
        use base64::{Engine as _, engine::general_purpose};
        
        let decoded = general_purpose::STANDARD.decode(base64_str)
            .map_err(|_| KnishIOError::SignatureMalformed)?;
        
        Ok(hex::encode(decoded))
    }

    /// Split string into chunks of specified size
    fn chunk_substr(string: &str, size: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut chars = string.chars();

        loop {
            let chunk: String = chars.by_ref().take(size).collect();
            if chunk.is_empty() {
                break;
            }
            chunks.push(chunk);
        }

        chunks
    }

    /// Reconstruct a Molecule from server-side GraphQL response data.
    ///
    /// Maps server field names to SDK field names:
    /// - `tokenSlug` / `token` → `token`
    /// - `metasJson` (JSON string) → `meta` (array of key-value pairs)
    /// - `bundleHash` → `bundle`
    ///
    /// Equivalent to CheckMolecule.fromServerData() in JavaScript.
    pub fn from_server_data(molecule_data: &Value) -> Result<Molecule> {
        let molecular_hash = molecule_data.get("molecularHash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let bundle_hash = molecule_data.get("bundleHash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let cell_slug = molecule_data.get("cellSlug")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let status = molecule_data.get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let created_at = molecule_data.get("createdAt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Map server atoms to SDK atom JSON format
        let mapped_atoms: Vec<Value> = molecule_data.get("atoms")
            .and_then(|v| v.as_array())
            .map(|atoms| {
                atoms.iter().map(|server_atom| {
                    // Parse metasJson into meta array
                    let meta = Self::parse_metas_json(server_atom);

                    // Map tokenSlug → token (server uses tokenSlug)
                    let token = server_atom.get("tokenSlug")
                        .and_then(|v| v.as_str())
                        .or_else(|| server_atom.get("token").and_then(|v| v.as_str()))
                        .unwrap_or_default();

                    // Map value to string (server may send as number)
                    let value = if let Some(v) = server_atom.get("value") {
                        if v.is_null() {
                            Value::Null
                        } else if let Some(s) = v.as_str() {
                            Value::String(s.to_string())
                        } else {
                            Value::String(v.to_string())
                        }
                    } else {
                        Value::Null
                    };

                    serde_json::json!({
                        "position": server_atom.get("position").and_then(|v| v.as_str()).unwrap_or_default(),
                        "walletAddress": server_atom.get("walletAddress").and_then(|v| v.as_str()).unwrap_or_default(),
                        "isotope": server_atom.get("isotope").and_then(|v| v.as_str()).unwrap_or_default(),
                        "token": token,
                        "value": value,
                        "batchId": server_atom.get("batchId").and_then(|v| v.as_str()),
                        "metaType": server_atom.get("metaType").and_then(|v| v.as_str()),
                        "metaId": server_atom.get("metaId").and_then(|v| v.as_str()),
                        "meta": meta,
                        "index": server_atom.get("index").and_then(|v| v.as_u64()),
                        "otsFragment": server_atom.get("otsFragment").and_then(|v| v.as_str()),
                        "createdAt": server_atom.get("createdAt").and_then(|v| v.as_str()),
                    })
                }).collect()
            })
            .unwrap_or_default();

        // Build molecule JSON in SDK format
        let molecule_json = serde_json::json!({
            "molecularHash": molecular_hash,
            "bundle": bundle_hash,
            "cellSlug": cell_slug,
            "status": status,
            "createdAt": created_at,
            "atoms": mapped_atoms,
        });

        let options = crate::types::MoleculeFromJsonOptions {
            include_validation_context: false,
            validate_structure: false,
            strict_mode: false,
        };

        Molecule::from_json(&molecule_json, options)
    }

    /// Parse metasJson field from a server atom into a meta array.
    ///
    /// Server atoms carry metadata as a JSON string in `metasJson`.
    /// This parses it into the `[{key, value}]` format the SDK expects.
    fn parse_metas_json(server_atom: &Value) -> Vec<Value> {
        let metas_json = match server_atom.get("metasJson").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return Vec::new(),
        };

        match serde_json::from_str::<Value>(metas_json) {
            Ok(Value::Array(arr)) => arr,
            Ok(Value::Object(obj)) => {
                // Object format {key1: val1, ...} → [{key, value}]
                obj.into_iter()
                    .map(|(key, value)| {
                        serde_json::json!({
                            "key": key,
                            "value": value.as_str().unwrap_or_default()
                        })
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Verify a molecule reconstructed from server-side GraphQL data.
    ///
    /// Reconstructs the molecule from server data and runs full verification
    /// (molecular hash + OTS signature). Returns a result object indicating
    /// success or failure with error details.
    ///
    /// Equivalent to CheckMolecule.verifyFromServerData() in JavaScript.
    pub fn verify_from_server_data(molecule_data: &Value) -> MoleculeIntegrityResult {
        let molecular_hash = molecule_data.get("molecularHash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match Self::try_verify_server_data(molecule_data) {
            Ok(()) => MoleculeIntegrityResult {
                molecular_hash,
                verified: true,
                error: None,
            },
            Err(e) => MoleculeIntegrityResult {
                molecular_hash,
                verified: false,
                error: Some(e.to_string()),
            },
        }
    }

    /// Internal helper that attempts verification and returns Result for error propagation.
    fn try_verify_server_data(molecule_data: &Value) -> Result<()> {
        let molecule = Self::from_server_data(molecule_data)?;
        let checker = CheckMolecule::new(&molecule)?;
        checker.verify(None)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{Atom, AtomCreateParams};
    use crate::types::Isotope;

    #[test]
    fn test_check_molecule_creation() {
        let mut molecule = Molecule::new();
        molecule.molecular_hash = Some("test_hash".to_string());
        
        // Add a test atom
        let atom = Atom::create(AtomCreateParams {
            position: Some("test_position".to_string()),
            wallet_address: Some("test_address".to_string()),
            isotope: Isotope::V,
            token: Some("TEST".to_string()),
            value: Some(100.0),
            index: Some(0),
            ..Default::default()
        });
        molecule.atoms.push(atom);

        let check_molecule = CheckMolecule::new(&molecule);
        assert!(check_molecule.is_ok());
    }

    #[test]
    fn test_missing_molecular_hash() {
        let molecule = Molecule::new();
        let check_molecule = CheckMolecule::new(&molecule);
        assert!(matches!(check_molecule.unwrap_err(), KnishIOError::MolecularHashMissing));
    }

    #[test]
    fn test_missing_atoms() {
        let mut molecule = Molecule::new();
        molecule.molecular_hash = Some("test_hash".to_string());
        
        let check_molecule = CheckMolecule::new(&molecule);
        assert!(matches!(check_molecule.unwrap_err(), KnishIOError::AtomsMissing));
    }

    #[test]
    fn test_chunk_substr() {
        let result = CheckMolecule::chunk_substr("abcdefgh", 3);
        assert_eq!(result, vec!["abc", "def", "gh"]);
    }

    #[test]
    fn test_from_server_data_maps_fields() {
        let server_data = serde_json::json!({
            "molecularHash": "abc123",
            "bundleHash": "bundle456",
            "cellSlug": "test_cell",
            "status": "accepted",
            "createdAt": "2026-01-01",
            "atoms": [
                {
                    "position": "pos1",
                    "walletAddress": "addr1",
                    "isotope": "M",
                    "tokenSlug": "USER",
                    "value": null,
                    "metaType": "TestMeta",
                    "metaId": "id1",
                    "metasJson": "[{\"key\":\"name\",\"value\":\"test\"}]",
                    "index": 0,
                    "otsFragment": "frag1"
                }
            ]
        });

        let molecule = CheckMolecule::from_server_data(&server_data).unwrap();
        assert_eq!(molecule.molecular_hash.as_deref(), Some("abc123"));
        assert_eq!(molecule.bundle.as_deref(), Some("bundle456"));
        assert_eq!(molecule.cell_slug.as_deref(), Some("test_cell"));
        assert_eq!(molecule.status.as_deref(), Some("accepted"));
        assert_eq!(molecule.atoms.len(), 1);
        assert_eq!(molecule.atoms[0].token, "USER");
        assert_eq!(molecule.atoms[0].meta_type.as_deref(), Some("TestMeta"));
        assert_eq!(molecule.atoms[0].ots_fragment.as_deref(), Some("frag1"));
        assert_eq!(molecule.atoms[0].meta.len(), 1);
        assert_eq!(molecule.atoms[0].meta[0].key, "name");
        assert_eq!(molecule.atoms[0].meta[0].value, "test");
    }

    #[test]
    fn test_from_server_data_token_slug_fallback() {
        // When tokenSlug is absent, should fall back to token
        let server_data = serde_json::json!({
            "molecularHash": "hash1",
            "atoms": [{
                "position": "p", "walletAddress": "a",
                "isotope": "V", "token": "TEST",
                "index": 0
            }]
        });

        let molecule = CheckMolecule::from_server_data(&server_data).unwrap();
        assert_eq!(molecule.atoms[0].token, "TEST");
    }

    #[test]
    fn test_from_server_data_metas_json_object_format() {
        // metasJson as object {key: value} instead of array
        let server_data = serde_json::json!({
            "molecularHash": "hash2",
            "atoms": [{
                "position": "p", "walletAddress": "a",
                "isotope": "M", "tokenSlug": "USER",
                "metasJson": "{\"color\":\"red\",\"size\":\"large\"}",
                "index": 0
            }]
        });

        let molecule = CheckMolecule::from_server_data(&server_data).unwrap();
        assert_eq!(molecule.atoms[0].meta.len(), 2);
        let meta_keys: Vec<&str> = molecule.atoms[0].meta.iter().map(|m| m.key.as_str()).collect();
        assert!(meta_keys.contains(&"color"));
        assert!(meta_keys.contains(&"size"));
    }

    #[test]
    fn test_verify_from_server_data_invalid_hash() {
        // Molecule with mismatched hash should fail verification
        let server_data = serde_json::json!({
            "molecularHash": "wrong_hash",
            "atoms": [{
                "position": "p", "walletAddress": "a",
                "isotope": "V", "token": "TEST",
                "value": "100", "index": 0
            }]
        });

        let result = CheckMolecule::verify_from_server_data(&server_data);
        assert!(!result.verified);
        assert!(result.error.is_some());
        assert_eq!(result.molecular_hash.as_deref(), Some("wrong_hash"));
    }

    #[test]
    fn test_verify_from_server_data_empty_atoms() {
        // Molecule with no atoms should fail
        let server_data = serde_json::json!({
            "molecularHash": "hash",
            "atoms": []
        });

        let result = CheckMolecule::verify_from_server_data(&server_data);
        assert!(!result.verified);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_integrity_report_empty_verified() {
        let report = IntegrityReport {
            verified: true,
            molecules: vec![],
        };
        assert!(report.verified);
        assert!(report.molecules.is_empty());
    }

    // ---------------------------------------------------------------------
    // Cross-isotope validators (isotope_a / isotope_b / isotope_f / isotope_p)
    //
    // These assert on the SPECIFIC error variant, not merely that something
    // errored. A validator that is never observed rejecting is indistinguishable
    // from one that does not exist — and an assertion that accepts any error can
    // pass on an unrelated guard firing first.
    // ---------------------------------------------------------------------

    use crate::types::MetaItem;

    /// Build a checkable molecule from atoms. A non-null molecular_hash is required
    /// or CheckMolecule::new rejects before any isotope method can run.
    fn checkable(atoms: Vec<Atom>) -> Molecule {
        let mut molecule = Molecule::new();
        molecule.molecular_hash = Some("0".repeat(64));
        molecule.atoms = atoms;
        molecule
    }

    fn v_atom(value: f64, address: &str, index: u32) -> Atom {
        Atom::create(AtomCreateParams {
            isotope: Isotope::V,
            position: Some("pos".to_string()),
            wallet_address: Some(address.to_string()),
            token: Some("TESTTOKEN".to_string()),
            value: Some(value),
            index: Some(index),
            ..Default::default()
        })
    }

    fn bundle_atom(isotope: Isotope, value: f64, meta_type: Option<&str>, meta_id: Option<&str>) -> Atom {
        Atom::create(AtomCreateParams {
            isotope,
            position: Some("pos".to_string()),
            wallet_address: Some("addr".to_string()),
            token: Some("TESTTOKEN".to_string()),
            value: Some(value),
            meta_type: meta_type.map(|s| s.to_string()),
            meta_id: meta_id.map(|s| s.to_string()),
            index: Some(0),
            ..Default::default()
        })
    }

    fn user_atom(isotope: Isotope, token: &str, meta_type: Option<&str>, meta_id: Option<&str>, meta: Vec<MetaItem>) -> Atom {
        Atom::create(AtomCreateParams {
            isotope,
            position: Some("pos".to_string()),
            wallet_address: Some("addr".to_string()),
            token: Some(token.to_string()),
            meta_type: meta_type.map(|s| s.to_string()),
            meta_id: meta_id.map(|s| s.to_string()),
            meta: Some(meta),
            index: Some(0),
            ..Default::default()
        })
    }

    fn meta_item(key: &str, value: &str) -> MetaItem {
        MetaItem { key: key.to_string(), value: value.to_string() }
    }

    #[test]
    fn test_isotope_b_rejects_wrong_meta_type() {
        let molecule = checkable(vec![bundle_atom(Isotope::B, 5.0, Some("wrongType"), Some("b1"))]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_b().unwrap_err(), KnishIOError::MetaMissing));
    }

    #[test]
    fn test_isotope_b_rejects_missing_meta_id() {
        let molecule = checkable(vec![bundle_atom(Isotope::B, 5.0, Some("walletBundle"), None)]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_b().unwrap_err(), KnishIOError::MetaMissing));
    }

    #[test]
    fn test_isotope_b_rejects_unbalanced_v_plus_b() {
        let molecule = checkable(vec![
            v_atom(-100.0, "src", 0),
            bundle_atom(Isotope::B, 30.0, Some("walletBundle"), Some("b1")),
        ]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_b().unwrap_err(), KnishIOError::TransferUnbalanced));
    }

    #[test]
    fn test_isotope_b_accepts_balanced_v_plus_b() {
        // The buffer deposit shape: V(-100) -> B(+30) -> V(+70)
        let molecule = checkable(vec![
            v_atom(-100.0, "src", 0),
            bundle_atom(Isotope::B, 30.0, Some("walletBundle"), Some("b1")),
            v_atom(70.0, "rem", 2),
        ]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(checker.isotope_b().is_ok());
    }

    #[test]
    fn test_isotope_f_rejects_negative_value() {
        let molecule = checkable(vec![bundle_atom(Isotope::F, -5.0, Some("walletBundle"), Some("f1"))]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_f().unwrap_err(), KnishIOError::TransferMalformed));
    }

    #[test]
    fn test_isotope_f_rejects_unbalanced_v_plus_f() {
        let molecule = checkable(vec![
            v_atom(-100.0, "src", 0),
            bundle_atom(Isotope::F, 30.0, Some("walletBundle"), Some("f1")),
        ]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_f().unwrap_err(), KnishIOError::TransferUnbalanced));
    }

    #[test]
    fn test_isotope_a_rejects_non_user_token() {
        let molecule = checkable(vec![user_atom(Isotope::A, "NOTUSER", Some("t"), Some("i"), vec![meta_item("action", "x")])]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_a().unwrap_err(), KnishIOError::WrongTokenType));
    }

    #[test]
    fn test_isotope_a_rejects_missing_action_meta() {
        let molecule = checkable(vec![user_atom(Isotope::A, "USER", Some("t"), Some("i"), vec![])]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_a().unwrap_err(), KnishIOError::MetaMissing));
    }

    #[test]
    fn test_isotope_p_rejects_non_user_token() {
        let molecule = checkable(vec![user_atom(Isotope::P, "NOTUSER", None, None, vec![meta_item("peerHost", "h")])]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_p().unwrap_err(), KnishIOError::WrongTokenType));
    }

    #[test]
    fn test_isotope_p_rejects_missing_peer_host() {
        let molecule = checkable(vec![user_atom(Isotope::P, "USER", None, None, vec![])]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_p().unwrap_err(), KnishIOError::MetaMissing));
    }

    /// The cross-isotope bypass must not become a blanket exemption: a plain V-only
    /// transfer that does not conserve value must still be rejected.
    #[test]
    fn test_bypass_not_over_broad_two_atom() {
        let molecule = checkable(vec![v_atom(-100.0, "src", 0), v_atom(30.0, "dst", 1)]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_v(None).unwrap_err(), KnishIOError::TransferUnbalanced));
    }

    #[test]
    fn test_bypass_not_over_broad_three_atom() {
        let molecule = checkable(vec![
            v_atom(-100.0, "src", 0),
            v_atom(30.0, "d1", 1),
            v_atom(10.0, "d2", 2),
        ]);
        let checker = CheckMolecule::new(&molecule).unwrap();
        assert!(matches!(checker.isotope_v(None).unwrap_err(), KnishIOError::TransferUnbalanced));
    }
}