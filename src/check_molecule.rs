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
use std::collections::HashMap;

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

    /// Validate Value isotope atoms (transfer validation)
    ///
    /// Equivalent to CheckMolecule.isotopeV() in JavaScript
    fn isotope_v(&self, sender_wallet: Option<&Wallet>) -> Result<bool> {
        let isotope_v = self.get_isotopes(&[Isotope::V]);

        if isotope_v.is_empty() {
            return Ok(true);
        }

        let first_atom = &self.molecule.atoms[0];

        // Handle simple 2-atom transfer case
        if first_atom.isotope == Isotope::V && isotope_v.len() == 2 {
            let end_atom = &isotope_v[isotope_v.len() - 1];

            if first_atom.token != end_atom.token {
                return Err(KnishIOError::TransferMismatched);
            }

            let first_value: f64 = first_atom.value.as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);

            let end_value: f64 = end_atom.value.as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);

            if end_value < 0.0 {
                return Err(KnishIOError::TransferMalformed);
            }

            // Check that the two atoms balance to zero
            let sum = first_value + end_value;
            if sum != 0.0 {
                return Err(KnishIOError::TransferUnbalanced);
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

        // All atoms must sum to zero for a balanced transaction
        if sum != 0.0 {
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

            let remainder = sender.balance + value;

            // Is there enough balance to send?
            if remainder < 0.0 {
                return Err(KnishIOError::TransferBalance);
            }

            // Does the remainder match what should be there in the source wallet, if provided?
            if remainder != sum {
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
    fn ots(&self) -> Result<bool> {
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
}