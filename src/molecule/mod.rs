//! Molecule module for KnishIO SDK
//!
//! This module provides the Molecule struct and related functionality for creating
//! molecular transactions. The implementation maintains 100% compatibility with
//! the JavaScript SDK, particularly the critical one-time signature algorithm.

pub mod builder;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::atom::{Atom, AtomCreateParams, WalletInfo};
use crate::wallet::Wallet;
use crate::crypto::{shake256, generate_bundle_hash};
use crate::types::{Isotope, MetaItem};
use crate::error::{KnishIOError, Result};
use crate::check_molecule::CheckMolecule;
use base64::{Engine as _, engine::general_purpose};

// Re-export the type-safe builder for convenience
pub use builder::{TypeSafeMoleculeBuilder, ValueAtomParams, MetaAtomParams, IdentityAtomParams, TokenRequestAtomParams};

/// Helper function to chunk a string into pieces of specified size
/// Equivalent to JavaScript's chunkSubstr function
fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut chars = s.chars();
    
    loop {
        let chunk: String = chars.by_ref().take(chunk_size).collect();
        if chunk.is_empty() {
            break;
        }
        chunks.push(chunk);
    }
    
    chunks
}

/// Represents a molecular transaction containing multiple atomic operations
///
/// Molecules are the fundamental units of transaction on the KnishIO distributed ledger,
/// containing one or more atoms that represent individual operations. This implementation
/// ensures exact compatibility with the JavaScript SDK, especially for cryptographic operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Molecule {
    /// Molecular hash computed from atoms
    pub molecular_hash: Option<String>,
    
    /// Bundle hash - 64-character hexadecimal user identifier
    pub bundle: Option<String>,
    
    /// Creation timestamp
    pub created_at: String,
    
    /// Status of the molecule
    pub status: Option<String>,
    
    /// Cell slug for sharding (optional)
    pub cell_slug: Option<String>,
    
    /// Original cell slug (optional)
    pub cell_slug_origin: Option<String>,
    
    /// Version identifier (optional)
    pub version: Option<String>,
    
    /// Atoms contained in this molecule
    pub atoms: Vec<Atom>,
    
    /// Secret for cryptographic operations (not serialized)
    #[serde(skip)]
    pub secret: Option<String>,
    
    /// Source wallet for this transaction (needed for cross-SDK validation)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sourceWallet")]
    pub source_wallet: Option<Wallet>,
    
    /// Remainder wallet for change (needed for cross-SDK validation)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "remainderWallet")]
    pub remainder_wallet: Option<Wallet>,
}

impl Molecule {
    /// Create a new empty Molecule instance
    pub fn new() -> Self {
        let timestamp = Self::generate_timestamp();
        
        Molecule {
            molecular_hash: None,
            bundle: None,
            created_at: timestamp,
            status: None,
            cell_slug: None,
            cell_slug_origin: None,
            version: None,
            atoms: Vec::new(),
            secret: None,
            source_wallet: None,
            remainder_wallet: None,
        }
    }
    
    /// Generate timestamp for molecule creation
    /// Use environment variable for deterministic testing
    fn generate_timestamp() -> String {
        if let Ok(fixed_time) = std::env::var("KNISHIO_FIXED_TIMESTAMP") {
            fixed_time
        } else {
            // JavaScript: String(+new Date()) - milliseconds since epoch
            chrono::Utc::now().timestamp_millis().to_string()
        }
    }
    
    /// Create a new Molecule instance with parameters
    /// # Arguments
    /// * `secret` - 2048-character biometric hash (optional)
    /// * `bundle` - 64-character hexadecimal user identifier (optional)
    /// * `source_wallet` - Source wallet for transactions (optional)
    /// * `remainder_wallet` - Remainder wallet for change (optional)
    /// * `cell_slug` - Cell slug for sharding (optional)
    /// * `version` - Version identifier (optional)
    pub fn with_params(
        secret: Option<String>,
        bundle: Option<String>,
        source_wallet: Option<Wallet>,
        remainder_wallet: Option<Wallet>,
        cell_slug: Option<String>,
        version: Option<String>,
    ) -> Self {
        let timestamp = Self::generate_timestamp();
        
        // Create remainder wallet if source wallet provided but no remainder wallet
        let final_remainder_wallet = if remainder_wallet.is_some() {
            remainder_wallet
        } else if let Some(ref source) = source_wallet {
            if let Some(ref secret_str) = secret {
                // Create remainder wallet from source wallet
                Wallet::create(
                    Some(secret_str),
                    bundle.as_deref(),
                    &source.token,
                    source.batch_id.as_deref(),
                    source.characters.as_deref(),
                ).ok()
            } else {
                None
            }
        } else {
            None
        };
        
        Molecule {
            molecular_hash: None,
            bundle,
            created_at: timestamp,
            status: None,
            cell_slug: cell_slug.clone(),
            cell_slug_origin: cell_slug,
            version,
            atoms: Vec::new(),
            secret,
            source_wallet,
            remainder_wallet: final_remainder_wallet,
        }
    }
    
    /// Convert JSON string to Molecule object (matches JS Molecule.jsonToObject)
    /// # Arguments
    /// * `json` - JSON string representation of a Molecule
    /// Result containing the deserialized Molecule or an error
    pub fn json_to_object(json: &str) -> Result<Molecule> {
        let mut molecule: Molecule = serde_json::from_str(json)?;
        
        if molecule.atoms.is_empty() {
            return Err(KnishIOError::AtomsMissing);
        }
        
        // Validate required atom properties
        for atom in &molecule.atoms {
            if atom.isotope != Isotope::R {
                if atom.position.is_empty() || atom.wallet_address.is_empty() {
                    return Err(KnishIOError::AtomsMissing);
                }
            }
        }
        
        // Sort atoms by index
        molecule.atoms = Atom::sort_atoms(&molecule.atoms);
        
        Ok(molecule)
    }
    
    /// Convert molecular hash to base-17 enumerated form
    /// Maps hex characters to integer values:
    /// 0=-8, 1=-7, 2=-6, 3=-5, 4=-4, 5=-3, 6=-2, 7=-1, 8=0, 9=1, a=2, b=3, c=4, d=5, e=6, f=7, g=8
    /// # Arguments
    /// * `hash` - Hexadecimal hash string
    /// Vector of mapped integer values
    pub fn enumerate(hash: &str) -> Vec<i8> {
        let mapped: HashMap<char, i8> = [
            ('0', -8), ('1', -7), ('2', -6), ('3', -5), ('4', -4), ('5', -3), ('6', -2), ('7', -1),
            ('8', 0), ('9', 1), ('a', 2), ('b', 3), ('c', 4), ('d', 5), ('e', 6), ('f', 7), ('g', 8)
        ].iter().cloned().collect();
        
        hash.to_lowercase()
            .chars()
            .filter_map(|c| mapped.get(&c).copied())
            .collect()
    }
    
    /// Normalize enumerated hash to ensure sum equals zero
    /// This ensures exactly 50% of the WOTS+ key is leaked with each usage,
    /// ensuring predictable key safety.
    /// # Arguments
    /// * `mapped_hash_array` - Enumerated hash values
    /// Normalized array where sum equals zero
    pub fn normalize(mut mapped_hash_array: Vec<i8>) -> Vec<i8> {
        let mut total: i32 = mapped_hash_array.iter().map(|&x| x as i32).sum();
        let total_condition = total < 0;
        
        while total != 0 {
            for value in mapped_hash_array.iter_mut() {
                let condition = if total_condition {
                    *value < 8
                } else {
                    *value > -8
                };
                
                if condition {
                    if total_condition {
                        *value += 1;
                        total += 1;
                    } else {
                        *value -= 1;
                        total -= 1;
                    }
                    
                    if total == 0 {
                        break;
                    }
                }
            }
        }
        
        mapped_hash_array
    }
    
    /// Filter atoms by isotope type(s)
    /// # Arguments
    /// * `isotopes` - Single isotope or vector of isotopes to filter by
    /// * `atoms` - Vector of atoms to filter
    /// Filtered vector of atoms
    pub fn isotope_filter(isotopes: &[Isotope], atoms: &[Atom]) -> Vec<Atom> {
        atoms.iter()
            .filter(|atom| isotopes.contains(&atom.isotope))
            .cloned()
            .collect()
    }
    
    /// Generate next atomic index for this molecule
    /// Next available index
    pub fn generate_next_atom_index(atoms: &[Atom]) -> u32 {
        atoms.len() as u32
    }
    
    /// Add an atom to this molecule
    /// # Arguments
    /// * `atom` - Atom to add to the molecule
    pub fn add_atom(&mut self, mut atom: Atom) {
        // Reset molecular hash when atoms change
        self.molecular_hash = None;
        
        // Set atom's index and version
        atom.index = Some(self.generate_index());
        if let Some(ref version) = self.version {
            atom.version = Some(version.clone());
        }
        
        // Add atom to collection
        self.atoms.push(atom);
    }
    
    /// Add a ContinuID atom for identity continuity
    pub fn add_continuid_atom(&mut self) -> Result<()> {
        if let Some(ref remainder_wallet) = self.remainder_wallet {
            let params = AtomCreateParams {
                isotope: Isotope::I,
                wallet_info: Some(WalletInfo {
                    position: remainder_wallet.position.clone().unwrap_or_default(),
                    address: remainder_wallet.address.clone().unwrap_or_default(),
                    token: remainder_wallet.token.clone(),
                    batch_id: remainder_wallet.batch_id.clone(),
                }),
                meta_type: Some("walletBundle".to_string()),
                meta_id: remainder_wallet.bundle.clone(),
                ..Default::default()
            };
            
            let atom = Atom::create(params);
            self.add_atom(atom);
        }
        
        Ok(())
    }
    
    /// Sign the molecule with one-time signature
    /// # Arguments
    /// * `bundle` - Bundle hash for non-anonymous signing
    /// * `anonymous` - Whether to sign anonymously
    /// * `compressed` - Whether to compress signature with Base64
    /// Result containing the last position or an error
    pub fn sign(&mut self, bundle: Option<String>, anonymous: bool, compressed: bool) -> Result<Option<String>> {
        // Check if we have atoms
        if self.atoms.is_empty() {
            return Err(KnishIOError::AtomsMissing);
        }
        
        // Derive the user's bundle
        if !anonymous && self.bundle.is_none() {
            if let Some(bundle_hash) = bundle {
                self.bundle = Some(bundle_hash);
            } else if let Some(ref secret) = self.secret {
                self.bundle = Some(generate_bundle_hash(secret));
            }
        }
        
        // Hash atoms to get molecular hash (use base17 as default like JS)
        self.molecular_hash = Some(Atom::hash_atoms(&self.atoms, "base17")?);
        
        // Get signing atom (first atom)
        let signing_atom = &self.atoms[0];
        
        // Get signing position
        let signing_position = signing_atom.position.clone();
        
        if signing_position.is_empty() {
            return Err(KnishIOError::SignatureMalformed);
        }
        
        // Generate the private signing key for this molecule
        if let Some(ref secret) = self.secret {
            let key = Wallet::generate_key(secret, &signing_atom.token, &signing_atom.position);
            
            // Subdivide key into 16 segments of 128 characters each
            let key_chunks = chunk_string(&key, 128);
            
            // Convert molecular hash to numeric notation and normalize
            let normalized_hash = self.normalized_hash()?;
            
            // Build one-time signature
            let mut signature_fragments = String::new();
            
            for (index, chunk) in key_chunks.iter().enumerate() {
                if index >= normalized_hash.len() {
                    break;
                }
                
                let mut working_chunk = chunk.clone();
                // Calculate iterations: 8 - value where value is -8 to 8
                // This gives us 0 to 16 iterations
                let iterations = (8 - normalized_hash[index] as i32) as usize;
                
                for _ in 0..iterations {
                    working_chunk = shake256(&working_chunk, 512);
                }
                
                signature_fragments.push_str(&working_chunk);
            }
            
            // Compress signature if requested (hex to base64)
            if compressed {
                // Convert hex string to bytes, then encode as base64
                let bytes = hex::decode(&signature_fragments)
                    .map_err(|_| KnishIOError::SignatureMalformed)?;
                signature_fragments = general_purpose::STANDARD.encode(bytes);
            }
            
            // Chunk signature across multiple atoms (string-based chunking)
            let chunk_size = (signature_fragments.len() as f64 / self.atoms.len() as f64).ceil() as usize;
            let chunked_signature = chunk_string(&signature_fragments, chunk_size);
            
            let mut last_position: Option<String> = None;
            
            // Assign signature fragments to atoms
            for (chunk_count, chunk) in chunked_signature.iter().enumerate() {
                if chunk_count < self.atoms.len() {
                    self.atoms[chunk_count].ots_fragment = Some(chunk.clone());
                    last_position = Some(self.atoms[chunk_count].position.clone());
                }
            }
            
            Ok(last_position)
        } else {
            Err(KnishIOError::SignatureMalformed)
        }
    }
    
    /// Get normalized hash for signing
    /// Normalized hash array for one-time signature
    pub fn normalized_hash(&self) -> Result<Vec<i8>> {
        if let Some(ref hash) = self.molecular_hash {
            let enumerated = Self::enumerate(hash);
            Ok(Self::normalize(enumerated))
        } else {
            Err(KnishIOError::MolecularHashMissing)
        }
    }
    
    /// Comprehensive molecule validation
    /// Uses CheckMolecule class for thorough validation including signature verification,
    /// balance checking, isotope validation, and policy compliance.
    /// # Arguments
    /// * `sender_wallet` - Optional sender wallet for balance validation
    /// True if all validations pass, error otherwise
    pub fn check(&self, sender_wallet: Option<&Wallet>) -> Result<bool> {
        use crate::check_molecule::CheckMolecule;
        
        let check_molecule = CheckMolecule::new(self)?;
        check_molecule.verify(sender_wallet)
    }
    
    /// Generate next atomic index for this molecule
    pub fn generate_index(&self) -> u32 {
        Self::generate_next_atom_index(&self.atoms)
    }
    
    /// Get atoms filtered by isotope type(s)
    pub fn get_isotopes(&self, isotopes: &[Isotope]) -> Vec<Atom> {
        Self::isotope_filter(isotopes, &self.atoms)
    }
    
    /// Initialize value transfer molecule
    /// # Arguments
    /// * `recipient_wallet` - Wallet to receive tokens
    /// * `amount` - Amount to transfer
    pub fn init_value(&mut self, recipient_wallet: &Wallet, amount: f64) -> Result<()> {
        // Extract needed values before making mutable borrows
        let (source_balance, source_wallet_info, remainder_wallet_info) = {
            if let Some(ref source_wallet) = self.source_wallet {
                if source_wallet.balance - amount < 0.0 {
                    return Err(KnishIOError::BalanceInsufficient);
                }
                
                let source_info = WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                };
                
                let remainder_info = if let Some(ref remainder_wallet) = self.remainder_wallet {
                    Some(WalletInfo {
                        position: remainder_wallet.position.clone().unwrap_or_default(),
                        address: remainder_wallet.address.clone().unwrap_or_default(),
                        token: remainder_wallet.token.clone(),
                        batch_id: remainder_wallet.batch_id.clone(),
                    })
                } else {
                    None
                };
                
                (source_wallet.balance, Some(source_info), remainder_info)
            } else {
                return Ok(());
            }
        };
        
        // Now we can safely make mutable borrows
        if let Some(source_info) = source_wallet_info {
            // Remove FULL BALANCE from source (JavaScript UTXO pattern)
            let source_params = AtomCreateParams {
                isotope: Isotope::V,
                wallet_info: Some(source_info),
                value: Some(-source_balance),  // Debit full balance, not just transfer amount
                ..Default::default()
            };
            self.add_atom(Atom::create(source_params));
            
            // Add tokens to recipient
            let recipient_params = AtomCreateParams {
                isotope: Isotope::V,
                wallet_info: Some(WalletInfo {
                    position: recipient_wallet.position.clone().unwrap_or_default(),
                    address: recipient_wallet.address.clone().unwrap_or_default(),
                    token: recipient_wallet.token.clone(),
                    batch_id: recipient_wallet.batch_id.clone(),
                }),
                value: Some(amount),
                meta_type: Some("walletBundle".to_string()),
                meta_id: recipient_wallet.bundle.clone(),
                ..Default::default()
            };
            self.add_atom(Atom::create(recipient_params));
            
            // Add remainder atom
            if let Some(remainder_info) = remainder_wallet_info {
                let remainder_params = AtomCreateParams {
                    isotope: Isotope::V,
                    wallet_info: Some(remainder_info),
                    value: Some(source_balance - amount),
                    meta_type: Some("walletBundle".to_string()),
                    meta_id: self.remainder_wallet.as_ref().and_then(|w| w.bundle.clone()),
                    ..Default::default()
                };
                self.add_atom(Atom::create(remainder_params));
            }
        }
        
        Ok(())
    }
    
    /// Initialize token creation molecule
    /// # Arguments
    /// * `recipient_wallet` - Wallet to receive new tokens
    /// * `amount` - Amount of tokens to create
    /// * `meta` - Token metadata
    pub fn init_token_creation(&mut self, recipient_wallet: &Wallet, amount: f64, meta: Vec<MetaItem>) -> Result<()> {
        if let Some(ref source_wallet) = self.source_wallet {
            let params = AtomCreateParams {
                isotope: Isotope::C,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                value: Some(amount),
                meta_type: Some("token".to_string()),
                meta_id: Some(recipient_wallet.token.clone()),
                meta: Some(meta),
                batch_id: recipient_wallet.batch_id.clone(),
                ..Default::default()
            };
            
            self.add_atom(Atom::create(params));
        }
        
        Ok(())
    }
    
    /// Initialize wallet creation molecule
    /// # Arguments
    /// * `wallet` - Wallet to create
    /// * `atom_meta` - Wallet metadata
    pub fn init_wallet_creation(&mut self, wallet: &Wallet, atom_meta: Vec<MetaItem>) -> Result<()> {
        let params = AtomCreateParams {
            isotope: Isotope::C,
            wallet_info: Some(WalletInfo {
                position: wallet.position.clone().unwrap_or_default(),
                address: wallet.address.clone().unwrap_or_default(),
                token: wallet.token.clone(),
                batch_id: wallet.batch_id.clone(),
            }),
            meta_type: Some("wallet".to_string()),
            meta_id: wallet.bundle.clone(),
            meta: Some(atom_meta),
            ..Default::default()
        };
        
        self.add_atom(Atom::create(params));
        self.add_continuid_atom()?;
        
        Ok(())
    }
    
    /// Initialize shadow wallet claim molecule
    /// # Arguments
    /// * `wallet` - Shadow wallet to claim
    pub fn init_shadow_wallet_claim(&mut self, wallet: &Wallet) -> Result<()> {
        let params = AtomCreateParams {
            isotope: Isotope::V,
            wallet_info: Some(WalletInfo {
                position: wallet.position.clone().unwrap_or_default(),
                address: wallet.address.clone().unwrap_or_default(),
                token: wallet.token.clone(),
                batch_id: wallet.batch_id.clone(),
            }),
            value: Some(wallet.balance),
            ..Default::default()
        };
        
        self.add_atom(Atom::create(params));
        
        if let Some(ref remainder_wallet) = self.remainder_wallet {
            let remainder_params = AtomCreateParams {
                isotope: Isotope::V,
                wallet_info: Some(WalletInfo {
                    position: remainder_wallet.position.clone().unwrap_or_default(),
                    address: remainder_wallet.address.clone().unwrap_or_default(),
                    token: remainder_wallet.token.clone(),
                    batch_id: remainder_wallet.batch_id.clone(),
                }),
                value: Some(remainder_wallet.balance),
                meta_type: Some("walletBundle".to_string()),
                meta_id: remainder_wallet.bundle.clone(),
                ..Default::default()
            };
            self.add_atom(Atom::create(remainder_params));
        }
        
        Ok(())
    }
    
    /// Initialize identifier creation molecule
    /// # Arguments
    /// * `identifier_type` - Type of identifier
    /// * `contact` - Contact information
    /// * `code` - Verification code
    pub fn init_identifier_creation(&mut self, identifier_type: &str, contact: &str, code: &str) -> Result<()> {
        if let Some(ref source_wallet) = self.source_wallet {
            let mut meta = Vec::new();
            meta.push(MetaItem::new("contact", contact));
            meta.push(MetaItem::new("code", code));
            
            let params = AtomCreateParams {
                isotope: Isotope::C,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                meta_type: Some("identifier".to_string()),
                meta_id: Some(identifier_type.to_string()),
                meta: Some(meta),
                ..Default::default()
            };
            
            self.add_atom(Atom::create(params));
            self.add_continuid_atom()?;
        }
        
        Ok(())
    }
    
    /// Initialize metadata molecule
    /// # Arguments
    /// * `meta` - Metadata key-value pairs
    /// * `meta_type` - Type of metadata
    /// * `meta_id` - Metadata identifier
    /// * `policy` - Access policy (optional)
    pub fn init_meta(&mut self, meta: Vec<MetaItem>, meta_type: &str, meta_id: &str, policy: Option<&str>) -> Result<()> {
        if let Some(ref source_wallet) = self.source_wallet {
            let params = AtomCreateParams {
                isotope: Isotope::M,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                meta_type: Some(meta_type.to_string()),
                meta_id: Some(meta_id.to_string()),
                meta: Some(meta.clone()),
                ..Default::default()
            };
            
            self.add_atom(Atom::create(params));
            
            // Add ContinuID atom (I isotope) to match JavaScript canonical behavior
            self.add_continuid_atom()?;
        }
        
        Ok(())
    }
    
    /// Initialize token request molecule
    /// # Arguments
    /// * `token` - Token to request
    /// * `amount` - Amount to request
    /// * `meta_type` - Request metadata type
    /// * `meta_id` - Request metadata ID
    /// * `meta` - Request metadata
    /// * `batch_id` - Batch ID
    pub fn init_token_request(&mut self, token: &str, amount: f64, meta_type: &str, meta_id: &str, meta: Vec<MetaItem>, batch_id: Option<String>) -> Result<()> {
        if let Some(ref source_wallet) = self.source_wallet {
            let params = AtomCreateParams {
                isotope: Isotope::T,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: token.to_string(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                value: Some(amount),
                meta_type: Some(meta_type.to_string()),
                meta_id: Some(meta_id.to_string()),
                meta: Some(meta),
                batch_id,
                ..Default::default()
            };
            
            self.add_atom(Atom::create(params));
        }
        
        Ok(())
    }
    
    /// Initialize authorization molecule
    /// # Arguments
    /// * `meta` - Authorization metadata
    pub fn init_authorization(&mut self, meta: Vec<MetaItem>) -> Result<()> {
        if let Some(ref source_wallet) = self.source_wallet {
            let params = AtomCreateParams {
                isotope: Isotope::U,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                meta: Some(meta),
                ..Default::default()
            };
            
            self.add_atom(Atom::create(params));
            self.add_continuid_atom()?;
        }
        
        Ok(())
    }
    
    /// Burns some amount of tokens from a wallet (matches JS burnToken)
    /// # Arguments
    /// * `amount` - Amount to burn (must be positive)
    /// * `wallet_bundle` - Optional wallet bundle (not used in implementation)
    pub fn burn_token(&mut self, amount: f64, _wallet_bundle: Option<String>) -> Result<()> {
        if amount < 0.0 {
            return Err(KnishIOError::NegativeAmount);
        }
        
        // Extract all needed data from source_wallet first
        let (source_atom, source_balance) = if let Some(ref source_wallet) = self.source_wallet {
            if source_wallet.balance - amount < 0.0 {
                return Err(KnishIOError::BalanceInsufficient);
            }
            
            let balance = source_wallet.balance;
            
            // Remove tokens from source wallet
            let source_params = AtomCreateParams {
                isotope: Isotope::V,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                value: Some(-amount),
                ..Default::default()
            };
            (Some(Atom::create(source_params)), balance)
        } else {
            (None, 0.0)
        };
        
        // Add atoms after immutable borrow ends
        if let Some(atom) = source_atom {
            self.add_atom(atom);
            
            // Add remainder to remainder wallet
            if let Some(ref remainder_wallet) = self.remainder_wallet {
                let remainder_params = AtomCreateParams {
                    isotope: Isotope::V,
                    wallet_info: Some(WalletInfo {
                        position: remainder_wallet.position.clone().unwrap_or_default(),
                        address: remainder_wallet.address.clone().unwrap_or_default(),
                        token: remainder_wallet.token.clone(),
                        batch_id: remainder_wallet.batch_id.clone(),
                    }),
                    value: Some(source_balance - amount),
                    meta_type: Some("walletBundle".to_string()),
                    meta_id: remainder_wallet.bundle.clone(),
                    ..Default::default()
                };
                self.add_atom(Atom::create(remainder_params));
            }
        }
        
        Ok(())
    }
    
    /// Replenishes non-finite token supplies (matches JS replenishToken)
    /// # Arguments
    /// * `amount` - Amount to replenish (must be positive)
    /// * `units` - Token units to add (optional)
    pub fn replenish_token(&mut self, amount: f64, units: Option<Vec<String>>) -> Result<()> {
        if amount < 0.0 {
            return Err(KnishIOError::NegativeAmount);
        }
        
        if let Some(ref mut source_wallet) = self.source_wallet.clone() {
            // Handle token units if provided
            if let Some(_unit_list) = units {
                // For token units, merge with existing units
                // This is a simplified version - full implementation would need token unit handling
                source_wallet.balance = amount;
                if let Some(ref mut remainder_wallet) = self.remainder_wallet {
                    remainder_wallet.balance = source_wallet.balance + amount;
                }
            } else {
                // Update wallet balances for fungible tokens
                if let Some(ref mut remainder_wallet) = self.remainder_wallet {
                    remainder_wallet.balance = source_wallet.balance + amount;
                }
                source_wallet.balance = amount;
            }
            
            // Add atom to remove tokens from source
            let source_params = AtomCreateParams {
                isotope: Isotope::V,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                value: Some(source_wallet.balance),
                ..Default::default()
            };
            self.add_atom(Atom::create(source_params));
            
            // Add remainder atom
            if let Some(ref remainder_wallet) = self.remainder_wallet {
                let remainder_params = AtomCreateParams {
                    isotope: Isotope::V,
                    wallet_info: Some(WalletInfo {
                        position: remainder_wallet.position.clone().unwrap_or_default(),
                        address: remainder_wallet.address.clone().unwrap_or_default(),
                        token: remainder_wallet.token.clone(),
                        batch_id: remainder_wallet.batch_id.clone(),
                    }),
                    value: Some(remainder_wallet.balance),
                    meta_type: Some("walletBundle".to_string()),
                    meta_id: remainder_wallet.bundle.clone(),
                    ..Default::default()
                };
                self.add_atom(Atom::create(remainder_params));
            }
        }
        
        Ok(())
    }
    
    /// Add a policy atom for rules and permissions (matches JS addPolicyAtom)
    /// # Arguments
    /// * `meta_type` - Type of metadata
    /// * `meta_id` - Metadata identifier
    /// * `meta` - Metadata key-value pairs
    /// * `policy` - Policy rules (optional)
    pub fn add_policy_atom(
        &mut self,
        meta_type: &str,
        meta_id: &str,
        meta: Vec<MetaItem>,
        _policy: Option<&str>,
    ) -> Result<()> {
        if let Some(ref secret) = self.secret {
            if let Some(ref source_wallet) = self.source_wallet {
                // Create policy wallet for USER token
                let policy_wallet = Wallet::create(
                    Some(secret),
                    source_wallet.bundle.as_deref(),
                    "USER",
                    None,
                    None,
                )?;
                
                let final_meta = meta;
                // TODO: Add policy to meta if provided
                
                let params = AtomCreateParams {
                    isotope: Isotope::R,
                    wallet_info: Some(WalletInfo {
                        position: policy_wallet.position.clone().unwrap_or_default(),
                        address: policy_wallet.address.clone().unwrap_or_default(),
                        token: policy_wallet.token.clone(),
                        batch_id: policy_wallet.batch_id.clone(),
                    }),
                    meta_type: Some(meta_type.to_string()),
                    meta_id: Some(meta_id.to_string()),
                    meta: Some(final_meta),
                    ..Default::default()
                };
                
                self.add_atom(Atom::create(params));
            }
        }
        
        Ok(())
    }
    
    /// Initialize deposit buffer molecule (matches JS initDepositBuffer)
    /// # Arguments
    /// * `amount` - Amount to deposit
    /// * `trade_rates` - Trading rates (not implemented in this version)
    pub fn init_deposit_buffer(&mut self, amount: f64, _trade_rates: HashMap<String, f64>) -> Result<()> {
        // Extract all needed data from source_wallet first
        let atoms_to_add = if let Some(ref source_wallet) = self.source_wallet {
            if source_wallet.balance - amount < 0.0 {
                return Err(KnishIOError::BalanceInsufficient);
            }
            
            let source_balance = source_wallet.balance;
            let source_bundle = source_wallet.bundle.clone();
            let source_token = source_wallet.token.clone();
            let source_batch_id = source_wallet.batch_id.clone();
            let source_position = source_wallet.position.clone().unwrap_or_default();
            let source_address = source_wallet.address.clone().unwrap_or_default();
            
            let mut atoms = Vec::new();
            
            // Create buffer wallet
            if let Some(ref secret) = self.secret {
                let buffer_wallet = Wallet::create(
                    Some(secret),
                    self.bundle.as_deref(),
                    &source_token,
                    source_batch_id.as_deref(),
                    None,
                )?;
                // TODO: Set trade rates on buffer wallet
                
                // Remove tokens from source
                let source_params = AtomCreateParams {
                    isotope: Isotope::V,
                    wallet_info: Some(WalletInfo {
                        position: source_position,
                        address: source_address,
                        token: source_token.clone(),
                        batch_id: source_batch_id.clone(),
                    }),
                    value: Some(-amount),
                    ..Default::default()
                };
                atoms.push(Atom::create(source_params));
                
                // Add tokens to buffer wallet
                let buffer_params = AtomCreateParams {
                    isotope: Isotope::B,
                    wallet_info: Some(WalletInfo {
                        position: buffer_wallet.position.clone().unwrap_or_default(),
                        address: buffer_wallet.address.clone().unwrap_or_default(),
                        token: buffer_wallet.token.clone(),
                        batch_id: buffer_wallet.batch_id.clone(),
                    }),
                    value: Some(amount),
                    meta_type: Some("walletBundle".to_string()),
                    meta_id: source_bundle.clone(),
                    ..Default::default()
                };
                atoms.push(Atom::create(buffer_params));
                
                // Add remainder atom if remainder wallet exists
                if let Some(ref remainder_wallet) = self.remainder_wallet {
                    let remainder_params = AtomCreateParams {
                        isotope: Isotope::V,
                        wallet_info: Some(WalletInfo {
                            position: remainder_wallet.position.clone().unwrap_or_default(),
                            address: remainder_wallet.address.clone().unwrap_or_default(),
                            token: remainder_wallet.token.clone(),
                            batch_id: remainder_wallet.batch_id.clone(),
                        }),
                        value: Some(source_balance - amount),
                        meta_type: Some("walletBundle".to_string()),
                        meta_id: source_bundle,
                        ..Default::default()
                    };
                    atoms.push(Atom::create(remainder_params));
                }
            }
            
            atoms
        } else {
            Vec::new()
        };
        
        // Add all atoms after immutable borrow ends
        for atom in atoms_to_add {
            self.add_atom(atom);
        }
        
        Ok(())
    }
    
    /// Initialize withdraw buffer molecule (matches JS initWithdrawBuffer)
    /// # Arguments
    /// * `recipients` - Map of recipient bundle hashes to amounts
    /// * `signing_wallet` - Optional wallet for signing
    pub fn init_withdraw_buffer(
        &mut self,
        recipients: HashMap<String, f64>,
        _signing_wallet: Option<&Wallet>,
    ) -> Result<()> {
        // Calculate total amount from all recipients
        let total_amount: f64 = recipients.values().sum();
        
        // Extract all needed data from source_wallet first
        let atoms_to_add = if let Some(ref source_wallet) = self.source_wallet {
            if source_wallet.balance - total_amount < 0.0 {
                return Err(KnishIOError::BalanceInsufficient);
            }
            
            let source_balance = source_wallet.balance;
            let source_token = source_wallet.token.clone();
            let source_batch_id = source_wallet.batch_id.clone();
            let source_bundle = source_wallet.bundle.clone();
            let source_position = source_wallet.position.clone().unwrap_or_default();
            let source_address = source_wallet.address.clone().unwrap_or_default();
            
            let mut atoms = Vec::new();
            
            // Remove tokens from source
            let source_params = AtomCreateParams {
                isotope: Isotope::B,
                wallet_info: Some(WalletInfo {
                    position: source_position,
                    address: source_address,
                    token: source_token.clone(),
                    batch_id: source_batch_id.clone(),
                }),
                value: Some(-total_amount),
                meta_type: Some("walletBundle".to_string()),
                meta_id: source_bundle,
                ..Default::default()
            };
            atoms.push(Atom::create(source_params));
            
            // Add atoms for each recipient
            for (recipient_bundle, amount) in recipients {
                let recipient_params = AtomCreateParams {
                    isotope: Isotope::V,
                    wallet_info: None, // This is a shadow wallet transfer
                    token: Some(source_token.clone()),
                    value: Some(amount),
                    batch_id: source_batch_id.clone(),
                    meta_type: Some("walletBundle".to_string()),
                    meta_id: Some(recipient_bundle),
                    ..Default::default()
                };
                atoms.push(Atom::create(recipient_params));
            }
            
            // Add remainder atom if remainder wallet exists
            if let Some(ref remainder_wallet) = self.remainder_wallet {
                let remainder_params = AtomCreateParams {
                    isotope: Isotope::B,
                    wallet_info: Some(WalletInfo {
                        position: remainder_wallet.position.clone().unwrap_or_default(),
                        address: remainder_wallet.address.clone().unwrap_or_default(),
                        token: remainder_wallet.token.clone(),
                        batch_id: remainder_wallet.batch_id.clone(),
                    }),
                    value: Some(source_balance - total_amount),
                    meta_type: Some("walletBundle".to_string()),
                    meta_id: remainder_wallet.bundle.clone(),
                    ..Default::default()
                };
                atoms.push(Atom::create(remainder_params));
            }
            
            atoms
        } else {
            Vec::new()
        };
        
        // Add all atoms after immutable borrow ends
        for atom in atoms_to_add {
            self.add_atom(atom);
        }
        
        Ok(())
    }
    
    /// Create rule molecule (matches JS createRule)
    /// # Arguments
    /// * `meta_type` - Rule metadata type
    /// * `meta_id` - Rule metadata ID
    /// * `rule` - Rule definition as JSON string
    /// * `policy` - Policy definition (optional)
    pub fn create_rule(
        &mut self,
        meta_type: &str,
        meta_id: &str,
        rule: &str,
        _policy: Option<&str>,
    ) -> Result<()> {
        if let Some(ref source_wallet) = self.source_wallet {
            // Create atom meta with rules
            let mut rule_meta = Vec::new();
            rule_meta.push(MetaItem::new("rule", rule));
            
            // TODO: Add policy to meta if provided
            
            let params = AtomCreateParams {
                isotope: Isotope::R,
                wallet_info: Some(WalletInfo {
                    position: source_wallet.position.clone().unwrap_or_default(),
                    address: source_wallet.address.clone().unwrap_or_default(),
                    token: source_wallet.token.clone(),
                    batch_id: source_wallet.batch_id.clone(),
                }),
                meta_type: Some(meta_type.to_string()),
                meta_id: Some(meta_id.to_string()),
                meta: Some(rule_meta),
                ..Default::default()
            };
            
            self.add_atom(Atom::create(params));
            self.add_continuid_atom()?;
        }
        
        Ok(())
    }
    
    /// Fuse token units into a new token (matches JS fuseToken)
    /// # Arguments
    /// * `token_units` - Token units to fuse
    /// * `recipient_wallet` - Wallet to receive fused token
    pub fn fuse_token(&mut self, token_units: Vec<String>, recipient_wallet: &Wallet) -> Result<()> {
        let amount = token_units.len() as f64;
        
        // Extract all needed data from source_wallet first
        let atoms_to_add = if let Some(ref source_wallet) = self.source_wallet {
            if source_wallet.balance - amount < 0.0 {
                return Err(KnishIOError::BalanceInsufficient);
            }
            
            let source_balance = source_wallet.balance;
            let source_position = source_wallet.position.clone().unwrap_or_default();
            let source_address = source_wallet.address.clone().unwrap_or_default();
            let source_token = source_wallet.token.clone();
            let source_batch_id = source_wallet.batch_id.clone();
            
            let mut atoms = Vec::new();
            
            // Remove tokens from source wallet
            let source_params = AtomCreateParams {
                isotope: Isotope::V,
                wallet_info: Some(WalletInfo {
                    position: source_position,
                    address: source_address,
                    token: source_token,
                    batch_id: source_batch_id,
                }),
                value: Some(-amount),
                ..Default::default()
            };
            atoms.push(Atom::create(source_params));
            
            // Add F isotope for fused tokens creation
            let fuse_params = AtomCreateParams {
                isotope: Isotope::F,
                wallet_info: Some(WalletInfo {
                    position: recipient_wallet.position.clone().unwrap_or_default(),
                    address: recipient_wallet.address.clone().unwrap_or_default(),
                    token: recipient_wallet.token.clone(),
                    batch_id: recipient_wallet.batch_id.clone(),
                }),
                value: Some(1.0),
                meta_type: Some("walletBundle".to_string()),
                meta_id: recipient_wallet.bundle.clone(),
                ..Default::default()
            };
            atoms.push(Atom::create(fuse_params));
            
            // Add remainder atom if remainder wallet exists
            if let Some(ref remainder_wallet) = self.remainder_wallet {
                let remainder_params = AtomCreateParams {
                    isotope: Isotope::V,
                    wallet_info: Some(WalletInfo {
                        position: remainder_wallet.position.clone().unwrap_or_default(),
                        address: remainder_wallet.address.clone().unwrap_or_default(),
                        token: remainder_wallet.token.clone(),
                        batch_id: remainder_wallet.batch_id.clone(),
                    }),
                    value: Some(source_balance - amount),
                    meta_type: Some("walletBundle".to_string()),
                    meta_id: remainder_wallet.bundle.clone(),
                    ..Default::default()
                };
                atoms.push(Atom::create(remainder_params));
            }
            
            atoms
        } else {
            Vec::new()
        };
        
        // Add all atoms after immutable borrow ends
        for atom in atoms_to_add {
            self.add_atom(atom);
        }
        
        Ok(())
    }
    
    /// Get the molecular hash for this molecule
    /// Computes the molecular hash by sorting atoms and generating hash based on
    /// their contents, following the SDK Implementation Guide requirements.
    /// Result containing the molecular hash string
    pub fn get_molecular_hash(&self) -> Result<String> {
        if self.atoms.is_empty() {
            return Err(KnishIOError::AtomsMissing);
        }
        
        // Use the same hash algorithm as the existing implementation
        let hash_result = Atom::hash_atoms(&self.atoms, "hex")?;
        Ok(hash_result)
    }
    
    /// Sign the molecule using the secret (simplified interface for type-safe builder)
    /// This is a convenience method that wraps the existing sign method
    /// with sensible defaults for the type-safe builder.
    /// # Arguments
    /// * `secret` - The cryptographic secret to use for signing
    /// Result indicating success or failure
    pub async fn sign_with_secret(&mut self, secret: &str) -> Result<()> {
        // Store the secret for signing operations
        self.secret = Some(secret.to_string());
        
        // Use the existing sign method with default parameters
        let result = self.sign(
            self.bundle.clone(), // Use the existing bundle
            false,               // Not anonymous
            false,               // Not compressed
        )?;
        
        // Update molecular hash based on the signing result
        if result.is_some() {
            self.molecular_hash = result;
        }
        
        Ok(())
    }
    
    /// Verify the molecule's signature and structure
    ///
    /// This async method provides compatibility with async workflows.
    /// Internally delegates to the synchronous check() method.
    ///
    /// # Returns
    /// Result indicating whether the signature and structure are valid
    pub async fn verify(&self) -> Result<bool> {
        self.check(None)
    }

    /// Verify the molecule with a specific sender wallet for balance validation
    ///
    /// # Arguments
    /// * `sender_wallet` - Optional sender wallet for additional balance checks
    ///
    /// # Returns
    /// Result indicating whether the signature and structure are valid
    pub async fn verify_with_wallet(&self, sender_wallet: &Wallet) -> Result<bool> {
        self.check(Some(sender_wallet))
    }
    
    /// Check if the molecule contains atoms of a specific isotope type
    /// # Arguments
    /// * `isotope` - The isotope type to check for
    /// True if the molecule contains atoms of the specified isotope type
    pub fn has_isotope(&self, isotope: Isotope) -> bool {
        self.atoms.iter().any(|atom| atom.isotope == isotope)
    }

    /// Enhanced JSON serialization for cross-SDK compatibility (Rust 2025 best practices)
    /// 
    /// Provides clean serialization of molecules with validation context and OTS fragments.
    /// Follows JavaScript canonical format while using Rust language patterns.
    /// # Arguments
    /// * `options` - Serialization options
    /// Result containing JSON-serializable Value
    pub fn to_json(&self, options: crate::types::MoleculeJsonOptions) -> crate::error::Result<serde_json::Value> {
        // Security check in secure mode
        if options.secure_mode && self.secret.is_some() {
            return Err(crate::error::KnishIOError::custom("Cannot serialize molecule with secret in secure mode"));
        }

        // Core molecule properties (always included) - JavaScript SDK compatible format
        let mut serialized = serde_json::json!({
            "status": self.status,
            "molecularHash": self.molecular_hash,
            "createdAt": self.created_at,
            "cellSlug": self.cell_slug,
            "version": self.version,
            "bundle": self.bundle
        });

        // Only include cellSlugOrigin when not null (JavaScript SDK compatibility)
        if let Some(ref cell_slug_origin) = self.cell_slug_origin {
            serialized["cellSlugOrigin"] = serde_json::json!(cell_slug_origin);
        }

        // Serialize atoms array with optional OTS fragments
        let atom_options = crate::types::AtomJsonOptions {
            include_ots_fragments: options.include_ots_fragments,
            validate_fields: false,
        };
        
        let atoms_json: std::result::Result<Vec<serde_json::Value>, _> = self.atoms.iter()
            .map(|atom| atom.to_json(atom_options.clone()))
            .collect();
        serialized["atoms"] = serde_json::Value::Array(atoms_json?);

        // Validation context (essential for cross-SDK validation)
        if options.include_validation_context {
            if let Some(ref source_wallet) = self.source_wallet {
                serialized["sourceWallet"] = serde_json::json!({
                    "address": source_wallet.address,
                    "position": source_wallet.position,
                    "token": source_wallet.token,
                    "balance": source_wallet.balance,
                    "bundle": source_wallet.bundle,
                    "batchId": source_wallet.batch_id,
                    "characters": source_wallet.characters,
                    "pubkey": source_wallet.pubkey,
                    "tokenUnits": serde_json::Value::Array(vec![]),  // JavaScript SDK compatibility: always empty array
                    "tradeRates": serde_json::Value::Object(serde_json::Map::new()),  // JavaScript SDK compatibility: always empty object
                    "molecules": serde_json::Value::Object(serde_json::Map::new())   // JavaScript SDK compatibility: always empty object
                });
            }

            if let Some(ref remainder_wallet) = self.remainder_wallet {
                serialized["remainderWallet"] = serde_json::json!({
                    "address": remainder_wallet.address,
                    "position": remainder_wallet.position,
                    "token": remainder_wallet.token,
                    "balance": remainder_wallet.balance,
                    "bundle": remainder_wallet.bundle,
                    "batchId": remainder_wallet.batch_id,
                    "characters": remainder_wallet.characters,
                    "pubkey": remainder_wallet.pubkey,
                    "tokenUnits": serde_json::Value::Array(vec![]),  // JavaScript SDK compatibility: always empty array
                    "tradeRates": serde_json::Value::Object(serde_json::Map::new()),  // JavaScript SDK compatibility: always empty object  
                    "molecules": serde_json::Value::Object(serde_json::Map::new())   // JavaScript SDK compatibility: always empty object
                });
            }
        }

        Ok(serialized)
    }

    /// Enhanced JSON deserialization for cross-SDK compatibility (Rust 2025 best practices)
    /// Handles cross-SDK molecule deserialization with robust error handling.
    /// Essential for cross-platform molecule validation and compatibility testing.
    /// # Arguments
    /// * `json` - JSON Value to deserialize
    /// * `options` - Deserialization options
    /// Result containing reconstructed Molecule instance
    pub fn from_json(json: &serde_json::Value, options: crate::types::MoleculeFromJsonOptions) -> crate::error::Result<Self> {
        // Validate required fields in strict mode
        if options.strict_mode || options.validate_structure {
            if json.get("molecularHash").is_none() || !json.get("atoms").and_then(|a| a.as_array()).map_or(false, |a| !a.is_empty()) {
                return Err(crate::error::KnishIOError::custom("Invalid molecule data: missing molecularHash or atoms array"));
            }
        }

        // Create minimal molecule instance (never include secret from JSON)
        let mut molecule = Molecule::with_params(
            None, // secret: never from JSON
            json.get("bundle").and_then(|b| b.as_str()).map(|s| s.to_string()),
            None, // sourceWallet: reconstructed separately
            None, // remainderWallet: reconstructed separately
            json.get("cellSlug").and_then(|c| c.as_str()).map(|s| s.to_string()),
            json.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
        );

        // Populate core properties with graceful handling of missing fields
        if let Some(status) = json.get("status").and_then(|s| s.as_str()) {
            molecule.status = Some(status.to_string());
        }
        if let Some(hash) = json.get("molecularHash").and_then(|h| h.as_str()) {
            molecule.molecular_hash = Some(hash.to_string());
        }
        if let Some(created_at) = json.get("createdAt").and_then(|c| c.as_str()) {
            molecule.created_at = created_at.to_string();
        }
        // Handle cellSlugOrigin gracefully - may be missing in some SDKs (PHP/C)
        if let Some(cell_slug_origin) = json.get("cellSlugOrigin").and_then(|c| c.as_str()) {
            molecule.cell_slug_origin = Some(cell_slug_origin.to_string());
        } else {
            // Default to cellSlug if cellSlugOrigin is missing (PHP/C SDK compatibility)
            molecule.cell_slug_origin = molecule.cell_slug.clone();
        }

        // Reconstruct atoms array with proper Atom instances
        if let Some(atoms_array) = json.get("atoms").and_then(|a| a.as_array()) {
            molecule.atoms = Vec::new();
            for atom_data in atoms_array {
                let atom_options = crate::types::AtomFromJsonOptions::default();
                let atom = Atom::from_json(atom_data, atom_options)
                    .map_err(|e| crate::error::KnishIOError::custom(&format!("Failed to reconstruct atom: {}", e)))?;
                molecule.atoms.push(atom);
            }
        }

        // Reconstruct validation context if available and requested
        if options.include_validation_context {
            if let Some(source_wallet_data) = json.get("sourceWallet") {
                molecule.source_wallet = Some(reconstruct_wallet_from_json(source_wallet_data)?);
            }
            if let Some(remainder_wallet_data) = json.get("remainderWallet") {
                molecule.remainder_wallet = Some(reconstruct_wallet_from_json(remainder_wallet_data)?);
            }
        }

        Ok(molecule)
    }
}

impl Default for Molecule {
    fn default() -> Self {
        Molecule::with_params(None, None, None, None, None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MetaItem;

    #[test]
    fn test_molecule_creation() {
        let molecule = Molecule::with_params(
            Some("test-secret".to_string()),
            Some("test-bundle".to_string()),
            None,
            None,
            None,
            None,
        );
        
        assert_eq!(molecule.bundle, Some("test-bundle".to_string()));
        assert_eq!(molecule.secret, Some("test-secret".to_string()));
        assert!(molecule.atoms.is_empty());
        assert!(molecule.molecular_hash.is_none());
    }
    
    #[test]
    fn test_enumerate() {
        let hash = "0123456789abcdef";
        let enumerated = Molecule::enumerate(hash);
        
        assert_eq!(enumerated.len(), 16);
        assert_eq!(enumerated[0], -8); // '0' -> -8
        assert_eq!(enumerated[1], -7); // '1' -> -7
        assert_eq!(enumerated[9], 1);  // '9' -> 1
        assert_eq!(enumerated[10], 2); // 'a' -> 2
        assert_eq!(enumerated[15], 7); // 'f' -> 7
    }
    
    #[test]
    fn test_normalize() {
        let mapped = vec![-8, -7, -6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7];
        let normalized = Molecule::normalize(mapped);
        
        // Sum should be zero
        let sum: i32 = normalized.iter().map(|&x| x as i32).sum();
        assert_eq!(sum, 0);
    }
    
    #[test]
    fn test_generate_next_atom_index() {
        let atoms = vec![
            Atom::new("pos1", "addr1", Isotope::V, "TEST"),
            Atom::new("pos2", "addr2", Isotope::V, "TEST"),
        ];
        
        let next_index = Molecule::generate_next_atom_index(&atoms);
        assert_eq!(next_index, 2);
    }
    
    #[test]
    fn test_isotope_filter() {
        let atoms = vec![
            Atom::new("pos1", "addr1", Isotope::V, "TEST"),
            Atom::new("pos2", "addr2", Isotope::M, "TEST"),
            Atom::new("pos3", "addr3", Isotope::V, "TEST"),
        ];
        
        let filtered = Molecule::isotope_filter(&[Isotope::V], &atoms);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|atom| atom.isotope == Isotope::V));
    }
    
    #[test]
    fn test_add_atom() {
        let mut molecule = Molecule::default();
        let atom = Atom::new("pos1", "addr1", Isotope::V, "TEST");
        
        molecule.add_atom(atom);
        
        assert_eq!(molecule.atoms.len(), 1);
        assert_eq!(molecule.atoms[0].index, Some(0));
        assert!(molecule.molecular_hash.is_none()); // Should be reset
    }
    
    #[test]
    fn test_json_serialization() {
        let mut molecule = Molecule::default();
        molecule.bundle = Some("test-bundle".to_string());
        molecule.add_atom(Atom::new("pos1", "addr1", Isotope::V, "TEST"));
        
        let json = serde_json::to_string(&molecule).unwrap();
        assert!(json.contains("test-bundle"));
        assert!(json.contains("atoms"));
        
        // Secret should not be serialized
        assert!(!json.contains("secret"));
    }
    
    #[test]
    fn test_json_deserialization() {
        let json = r#"{
            "molecularHash": null,
            "bundle": "test-bundle",
            "createdAt": "1640995200000",
            "status": null,
            "cellSlug": null,
            "cellSlugOrigin": null,
            "version": null,
            "atoms": [
                {
                    "position": "pos1",
                    "walletAddress": "addr1",
                    "isotope": "V",
                    "token": "TEST",
                    "meta": [],
                    "createdAt": "1640995200000"
                }
            ]
        }"#;
        
        let molecule = Molecule::json_to_object(json).unwrap();
        assert_eq!(molecule.bundle, Some("test-bundle".to_string()));
        assert_eq!(molecule.atoms.len(), 1);
        assert_eq!(molecule.atoms[0].isotope, Isotope::V);
    }
    
    #[test]
    fn test_init_value() {
        let source_wallet = Wallet::create(
            Some("test-secret"), 
            None, 
            "TEST", 
            None, 
            None
        ).unwrap();
        
        let mut source_wallet = source_wallet;
        source_wallet.balance = 100.0;
        
        let recipient_wallet = Wallet::create(
            Some("test-secret2"), 
            None, 
            "TEST", 
            None, 
            None
        ).unwrap();
        
        let remainder_wallet = Wallet::create(
            Some("test-secret"), 
            None, 
            "TEST", 
            None, 
            None
        ).unwrap();
        
        let mut molecule = Molecule::with_params(
            Some("test-secret".to_string()),
            None,
            Some(source_wallet),
            Some(remainder_wallet),
            None,
            None,
        );
        
        molecule.init_value(&recipient_wallet, 50.0).unwrap();
        
        assert_eq!(molecule.atoms.len(), 3); // source, recipient, remainder
        assert_eq!(molecule.atoms[0].isotope, Isotope::V);
        assert_eq!(molecule.atoms[0].value, Some("-50".to_string()));
        assert_eq!(molecule.atoms[1].value, Some("50".to_string()));
        assert_eq!(molecule.atoms[2].value, Some("50".to_string()));
    }
    
    #[test]
    fn test_insufficient_balance() {
        let mut source_wallet = Wallet::create(
            Some("test-secret"), 
            None, 
            "TEST", 
            None, 
            None
        ).unwrap();
        source_wallet.balance = 10.0;
        
        let recipient_wallet = Wallet::create(
            Some("test-secret2"), 
            None, 
            "TEST", 
            None, 
            None
        ).unwrap();
        
        let mut molecule = Molecule::with_params(
            Some("test-secret".to_string()),
            None,
            Some(source_wallet),
            None,
            None,
            None,
        );
        
        let result = molecule.init_value(&recipient_wallet, 50.0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnishIOError::BalanceInsufficient));
    }
}

/// Helper function to reconstruct wallet from JSON data for validation context
/// 
/// Matches the JavaScript pattern of creating wallets with proper balances
/// for molecular validation.
fn reconstruct_wallet_from_json(wallet_data: &serde_json::Value) -> crate::error::Result<crate::wallet::Wallet> {
    let token = wallet_data.get("token")
        .and_then(|t| t.as_str())
        .unwrap_or("TEST");
        
    let position = wallet_data.get("position")
        .and_then(|p| p.as_str())
        .map(|s| s.to_string());
        
    let address = wallet_data.get("address")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string());
        
    // Handle balance as either integer or float (PHP uses integers, others use floats)
    let balance = wallet_data.get("balance")
        .and_then(|b| {
            if let Some(f) = b.as_f64() {
                Some(f)
            } else if let Some(i) = b.as_i64() {
                Some(i as f64)
            } else {
                None
            }
        })
        .unwrap_or(0.0);

    let bundle = wallet_data.get("bundle")
        .and_then(|b| b.as_str())
        .map(|s| s.to_string());

    let batch_id = wallet_data.get("batchId")
        .and_then(|b| b.as_str())
        .map(|s| s.to_string());

    // Provide default for characters if missing (PHP/C SDK compatibility)  
    let characters = wallet_data.get("characters")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .or_else(|| Some("BASE64".to_string())); // Default value for cross-SDK compatibility
    
    // Create wallet with minimal required information for validation
    // Handle cases where bundle is missing (PHP/C SDK compatibility)
    let mut wallet = if bundle.is_some() {
        // Normal case: use bundle
        crate::wallet::Wallet::create(
            None, // secret not needed for validation
            bundle.as_deref(),
            token,
            position.as_deref(), 
            characters.as_deref(),
        )?
    } else {
        // Special case: PHP/C SDKs may not include bundle in sourceWallet
        // Create wallet using new() method directly to bypass credential validation
        crate::wallet::Wallet::new(
            None, // secret
            None, // bundle (missing in PHP)
            Some(token),
            address.as_deref(),
            position.as_deref(),
            batch_id.as_deref(),
            characters.as_deref(),
        )?
    };
    
    // Set additional properties from JSON
    wallet.balance = balance;
    if let Some(addr) = address {
        wallet.address = Some(addr);
    }
    if let Some(pos) = position {
        wallet.position = Some(pos);
    }
    if let Some(batch) = batch_id {
        wallet.batch_id = Some(batch);
    }
    
    // Handle optional fields that might be missing in other SDK JSON (especially PHP/C)
    // Set default values to ensure compatibility
    if wallet.characters.is_none() {
        wallet.characters = Some("BASE64".to_string());
    }
    
    // Initialize empty collections for missing fields to match JavaScript structure
    if wallet.token_units.is_empty() {
        wallet.token_units = Vec::new(); // Already initialized as Vec::new() by default
    }
    
    if wallet.trade_rates.is_empty() {
        wallet.trade_rates = HashMap::new(); // Already initialized as HashMap::new() by default
    }
    
    if wallet.molecules.is_empty() {
        wallet.molecules = HashMap::new(); // Already initialized as HashMap::new() by default
    }
    
    // Extract optional pubkey if present (might be missing in some SDKs)
    if let Some(pubkey_val) = wallet_data.get("pubkey").and_then(|p| p.as_str()) {
        wallet.pubkey = Some(pubkey_val.to_string());
    }
    
    Ok(wallet)
}

// JavaScript-style convenience methods for cross-SDK validation
impl Molecule {
    /// Rust-style method (satisfies compiler warnings)
    pub fn to_json_string(&self) -> crate::error::Result<String> {
        self.toJSON()
    }
    
    /// Rust-style method (satisfies compiler warnings)  
    pub fn from_json_string(json: &str) -> crate::error::Result<Self> {
        Self::fromJSON(json)
    }
    
    /// JavaScript-style toJSON() convenience method
    /// Returns JSON string directly, matching JavaScript SDK pattern
    #[allow(non_snake_case)]
    pub fn toJSON(&self) -> crate::error::Result<String> {
        // Debug: Check molecule state before serialization
        eprintln!("DEBUG: Rust toJSON() called - atom_count: {}, molecular_hash: {:?}", 
                 self.atoms.len(), self.molecular_hash);
        
        let options = crate::types::MoleculeJsonOptions::default();
        let json_value = match self.to_json(options) {
            Ok(val) => {
                eprintln!("DEBUG: Rust to_json() succeeded, JSON keys: {:?}", 
                         val.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                val
            },
            Err(e) => {
                eprintln!("ERROR: Rust to_json() failed: {}", e);
                return Err(e);
            }
        };
        
        let json_string = serde_json::to_string(&json_value)
            .map_err(|e| crate::error::KnishIOError::custom(&format!("JSON serialization failed: {}", e)))?;
            
        eprintln!("DEBUG: Rust toJSON() output length: {}, first 100 chars: {}", 
                 json_string.len(), 
                 &json_string.chars().take(100).collect::<String>());
                 
        Ok(json_string)
    }
    
    /// JavaScript-style fromJSON() convenience method  
    /// Creates Molecule instance from JSON string, matching JavaScript SDK pattern
    #[allow(non_snake_case)]
    pub fn fromJSON(json: &str) -> crate::error::Result<Self> {
        let json_value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| crate::error::KnishIOError::custom(&format!("JSON parsing failed: {}", e)))?;
        
        // Use JavaScript-compatible default options for cross-SDK compatibility
        let options = crate::types::MoleculeFromJsonOptions {
            include_validation_context: true,
            validate_structure: false, // CRITICAL: Disable strict validation for cross-SDK compatibility
            strict_mode: false, // Critical: Allow flexibility for cross-SDK compatibility
        };
        
        Self::from_json(&json_value, options)
    }
    
    /// Enhanced JavaScript-style fromJSON() method with options
    /// Matches the JavaScript SDK's fromJSON(json, options) signature exactly
    #[allow(non_snake_case)]
    pub fn fromJSON_with_options(json: &str, include_validation_context: bool, validate_structure: bool, strict_mode: bool) -> crate::error::Result<Self> {
        let json_value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| crate::error::KnishIOError::custom(&format!("JSON parsing failed: {}", e)))?;
        
        let options = crate::types::MoleculeFromJsonOptions {
            include_validation_context,
            validate_structure,
            strict_mode,
        };
        
        Self::from_json(&json_value, options)
    }
}

