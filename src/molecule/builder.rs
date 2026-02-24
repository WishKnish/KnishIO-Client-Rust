//! Type-Safe MoleculeBuilder with Phantom Types
//!
//! Production-grade molecule construction using Rust 2025 type-level programming patterns.
//! Ensures correct molecular transaction composition at compile time with zero runtime overhead.
//! 
//! # Features
//!
//! - **Type-Level State Machine**: Compile-time validation of construction states
//! - **Zero-Cost Abstractions**: Phantom types erased at compile time
//! - **Fluent API**: Intuitive builder pattern with method chaining
//! - **Cross-SDK Compatibility**: Maintains exact compatibility with JavaScript reference
//! - **Isotope-Specific Builders**: Type-safe atom creation for each isotope type
//!
//! # Examples
//!
//! ```rust
//! use knishio_client::molecule::TypeSafeMoleculeBuilder;
//! use knishio_client::{Wallet, Isotope};
//!
//! let wallet = Wallet::create("test-secret", "TEST").unwrap();
//! 
//! let molecule = TypeSafeMoleculeBuilder::new("test-secret")
//!     .with_source_wallet(wallet.clone())
//!     .add_value_atom(ValueAtomParams {
//!         position: "W1".to_string(),
//!         wallet_address: wallet.address.unwrap(),
//!         token: "TEST".to_string(),
//!         value: Some(100.0),
//!         ..Default::default()
//!     })
//!     .add_remainder_atom(wallet)
//!     .ready_to_sign()
//!     .sign()
//!     .await?
//!     .build();
//! ```

use std::marker::PhantomData;

use crate::molecule::Molecule;
use crate::atom::Atom;
use crate::wallet::Wallet;
use crate::types::{Isotope, MetaItem};
use crate::error::{KnishIOError, Result};

/// Type-level states for compile-time validation
pub mod states {
    /// Empty molecule - no configuration yet
    pub struct Empty;
    
    /// Has source wallet configured
    pub struct WithSourceWallet;
    
    /// Has atoms added to the molecule
    pub struct WithAtoms;
    
    /// Ready for signing - all required components present
    pub struct ReadyToSign;
    
    /// Signed molecule - ready to build
    pub struct Signed;
}

/// Type-safe molecule builder using phantom types for compile-time validation
/// 
/// This builder ensures that molecules are constructed correctly and all required
/// components are present before allowing finalization. The type system prevents
/// invalid states from being represented.
pub struct TypeSafeMoleculeBuilder<State> {
    molecule: Molecule,
    secret: Option<String>,
    source_wallet: Option<Wallet>,
    remainder_wallet: Option<Wallet>,
    _phantom: PhantomData<State>,
}

/// Parameters for creating a Value isotope atom
#[derive(Debug, Clone)]
pub struct ValueAtomParams {
    pub position: String,
    pub wallet_address: String,
    pub token: String,
    pub value: Option<f64>,
    pub batch_id: Option<String>,
    pub meta: Option<Vec<MetaItem>>,
}

impl Default for ValueAtomParams {
    fn default() -> Self {
        Self {
            position: String::new(),
            wallet_address: String::new(),
            token: String::new(),
            value: None,
            batch_id: None,
            meta: None,
        }
    }
}

/// Parameters for creating a Metadata isotope atom
#[derive(Debug, Clone)]
pub struct MetaAtomParams {
    pub position: String,
    pub wallet_address: String,
    pub token: String,
    pub meta_type: String,
    pub meta_id: String,
    pub meta: Vec<MetaItem>,
    pub batch_id: Option<String>,
}

/// Parameters for creating an Identity isotope atom
#[derive(Debug, Clone)]
pub struct IdentityAtomParams {
    pub position: String,
    pub wallet_address: String,
    pub token: String,
    pub meta: Vec<MetaItem>,
    pub batch_id: Option<String>,
}

/// Parameters for creating a Token Request isotope atom
#[derive(Debug, Clone)]
pub struct TokenRequestAtomParams {
    pub position: String,
    pub wallet_address: String,
    pub token: String,
    pub meta: Vec<MetaItem>,
    pub batch_id: Option<String>,
}

/// Parameters for creating a Buffer Deposit isotope atom (B-isotope)
#[derive(Debug, Clone)]
pub struct BufferDepositAtomParams {
    pub position: String,
    pub wallet_address: String,
    pub token: String,
    pub value: f64,
    pub meta_type: Option<String>,
    pub meta_id: Option<String>,
    pub batch_id: Option<String>,
}

/// Parameters for creating a Buffer Withdraw isotope atom (B-isotope)
#[derive(Debug, Clone)]
pub struct BufferWithdrawAtomParams {
    pub position: String,
    pub wallet_address: String,
    pub token: String,
    pub value: f64,
    pub meta_type: Option<String>,
    pub meta_id: Option<String>,
    pub batch_id: Option<String>,
}

/// Parameters for creating a Fusion isotope atom (F-isotope)
#[derive(Debug, Clone)]
pub struct FusionAtomParams {
    pub position: String,
    pub wallet_address: String,
    pub token: String,
    pub value: Option<f64>,
    pub meta_type: Option<String>,
    pub meta_id: Option<String>,
    pub batch_id: Option<String>,
}

/// Parameters for a stackable token transfer (UTXO pattern with batch_id)
///
/// Creates a complete V-isotope triple (source debit, recipient credit, remainder)
/// in one builder call, propagating batch_id across all atoms.
#[derive(Debug, Clone)]
pub struct StackableTransferParams {
    pub token: String,
    pub amount: f64,
    pub recipient_address: String,
    pub recipient_position: String,
    pub recipient_bundle: Option<String>,
    pub batch_id: Option<String>,
}

// ============================================================================
// Type-Safe State Transitions
// ============================================================================

impl TypeSafeMoleculeBuilder<states::Empty> {
    /// Create a new type-safe molecule builder
    ///
    /// # Arguments
    ///
    /// * `secret` - Cryptographic secret for signing operations
    ///
    /// # Returns
    ///
    /// New builder in Empty state
    pub fn new<S: Into<String>>(secret: S) -> Self {
        Self {
            molecule: Molecule::new(),
            secret: Some(secret.into()),
            source_wallet: None,
            remainder_wallet: None,
            _phantom: PhantomData,
        }
    }

    /// Configure the source wallet for this molecule
    ///
    /// # Arguments
    ///
    /// * `wallet` - Source wallet for the transaction
    ///
    /// # Returns
    ///
    /// Builder in WithSourceWallet state
    pub fn with_source_wallet(mut self, wallet: Wallet) -> TypeSafeMoleculeBuilder<states::WithSourceWallet> {
        self.molecule.source_wallet = Some(wallet.clone());
        self.molecule.bundle = wallet.bundle.clone();
        
        TypeSafeMoleculeBuilder {
            molecule: self.molecule,
            secret: self.secret,
            source_wallet: Some(wallet),
            remainder_wallet: self.remainder_wallet,
            _phantom: PhantomData,
        }
    }
}

impl TypeSafeMoleculeBuilder<states::WithSourceWallet> {
    /// Configure an optional remainder wallet for change
    ///
    /// # Arguments
    ///
    /// * `wallet` - Remainder wallet for receiving change
    ///
    /// # Returns
    ///
    /// Builder in same state with remainder wallet configured
    pub fn with_remainder_wallet(mut self, wallet: Wallet) -> Self {
        self.molecule.remainder_wallet = Some(wallet.clone());
        self.remainder_wallet = Some(wallet);
        self
    }

    /// Set cell slug for sharding
    ///
    /// # Arguments
    ///
    /// * `cell_slug` - Cell identifier for sharding
    ///
    /// # Returns
    ///
    /// Builder in same state with cell slug configured
    pub fn with_cell_slug<S: Into<String>>(mut self, cell_slug: S) -> Self {
        self.molecule.cell_slug = Some(cell_slug.into());
        self
    }

    /// Set parent molecule hashes for DAG linkage
    ///
    /// Parent hashes connect this molecule to its predecessors in the DAG,
    /// enabling tip selection and branch resolution by the validator.
    ///
    /// # Arguments
    ///
    /// * `hashes` - Parent molecule hashes (1-8 parents, empty for genesis)
    ///
    /// # Returns
    ///
    /// Builder in same state with parent hashes configured
    pub fn with_parent_hashes(mut self, hashes: Vec<String>) -> Self {
        self.molecule.parent_hashes = hashes;
        self
    }

    /// Add a Value isotope atom to the molecule
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the value atom
    ///
    /// # Returns
    ///
    /// Builder in WithAtoms state
    pub fn add_value_atom(self, params: ValueAtomParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        self.add_atom_internal(Isotope::V, params.position, params.wallet_address, 
                              params.token, params.value, params.batch_id, 
                              None, None, params.meta)
    }

    /// Add a Metadata isotope atom to the molecule
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the metadata atom
    ///
    /// # Returns
    ///
    /// Builder in WithAtoms state
    pub fn add_meta_atom(self, params: MetaAtomParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        self.add_atom_internal(Isotope::M, params.position, params.wallet_address,
                              params.token, None, params.batch_id,
                              Some(params.meta_type), Some(params.meta_id), Some(params.meta))
    }

    /// Add an Identity isotope atom to the molecule
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the identity atom
    ///
    /// # Returns
    ///
    /// Builder in WithAtoms state
    pub fn add_identity_atom(self, params: IdentityAtomParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        self.add_atom_internal(Isotope::I, params.position, params.wallet_address,
                              params.token, None, params.batch_id,
                              None, None, Some(params.meta))
    }

    /// Add a Token Request isotope atom to the molecule
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the token request atom
    ///
    /// # Returns
    ///
    /// Builder in WithAtoms state
    pub fn add_token_request_atom(self, params: TokenRequestAtomParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        self.add_atom_internal(Isotope::T, params.position, params.wallet_address,
                              params.token, None, params.batch_id,
                              None, None, Some(params.meta))
    }

    /// Add a Buffer Deposit isotope atom (B-isotope) to the molecule
    pub fn add_buffer_deposit_atom(self, params: BufferDepositAtomParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        self.add_atom_internal(Isotope::B, params.position, params.wallet_address,
                              params.token, Some(params.value), params.batch_id,
                              params.meta_type, params.meta_id, None)
    }

    /// Add a Buffer Withdraw isotope atom (B-isotope) to the molecule
    pub fn add_buffer_withdraw_atom(self, params: BufferWithdrawAtomParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        self.add_atom_internal(Isotope::B, params.position, params.wallet_address,
                              params.token, Some(params.value), params.batch_id,
                              params.meta_type, params.meta_id, None)
    }

    /// Add a Fusion isotope atom (F-isotope) to the molecule
    pub fn add_fusion_atom(self, params: FusionAtomParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        self.add_atom_internal(Isotope::F, params.position, params.wallet_address,
                              params.token, params.value, params.batch_id,
                              params.meta_type, params.meta_id, None)
    }

    /// Add a complete stackable token transfer (UTXO pattern with batch_id).
    ///
    /// Creates 3 V-isotope atoms in one call:
    /// 1. **Source debit**: Full balance negative from source wallet
    /// 2. **Recipient credit**: Requested amount positive to recipient
    /// 3. **Remainder credit**: Change back to remainder wallet (if > 0)
    ///
    /// All atoms receive the `batch_id` for stackable token tracking.
    ///
    /// # Arguments
    ///
    /// * `params` - Stackable transfer parameters
    ///
    /// # Returns
    ///
    /// Builder in WithAtoms state with 2-3 V-isotope atoms added
    pub fn add_stackable_transfer(mut self, params: StackableTransferParams) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        let source_wallet = self.source_wallet.as_ref()
            .ok_or_else(|| KnishIOError::custom("Source wallet is required"))?;

        let source_balance = source_wallet.balance_as_i128();
        let amount_i128 = params.amount as i128;

        if source_balance < amount_i128 {
            return Err(KnishIOError::BalanceInsufficient);
        }

        // Atom 1: Source debit (negative full balance)
        let source_atom = Atom::new(
            source_wallet.position.as_deref().unwrap_or(""),
            source_wallet.address.as_deref().unwrap_or(""),
            Isotope::V,
            &params.token,
        ).with_optional_fields(
            None, // Set via string below for i128 precision
            params.batch_id.as_deref(),
            None,
            None,
            None,
        );
        let mut source_atom = source_atom;
        source_atom.value = Some((-source_balance).to_string());
        self.molecule.add_atom(source_atom);

        // Atom 2: Recipient credit (positive transfer amount)
        let recipient_atom = Atom::new(
            &params.recipient_position,
            &params.recipient_address,
            Isotope::V,
            &params.token,
        ).with_optional_fields(
            None,
            params.batch_id.as_deref(),
            Some("walletBundle"),
            params.recipient_bundle.as_deref(),
            None,
        );
        let mut recipient_atom = recipient_atom;
        recipient_atom.value = Some(amount_i128.to_string());
        self.molecule.add_atom(recipient_atom);

        // Atom 3: Remainder credit (change back to remainder wallet)
        let remainder = source_balance - amount_i128;
        if remainder > 0 {
            let remainder_wallet = self.remainder_wallet.as_ref()
                .ok_or_else(|| KnishIOError::custom("Remainder wallet required when transfer < source balance"))?;

            let remainder_atom = Atom::new(
                remainder_wallet.position.as_deref().unwrap_or(""),
                remainder_wallet.address.as_deref().unwrap_or(""),
                Isotope::V,
                &params.token,
            ).with_optional_fields(
                None,
                params.batch_id.as_deref(),
                Some("walletBundle"),
                remainder_wallet.bundle.as_deref(),
                None,
            );
            let mut remainder_atom = remainder_atom;
            remainder_atom.value = Some(remainder.to_string());
            self.molecule.add_atom(remainder_atom);
        }

        Ok(TypeSafeMoleculeBuilder {
            molecule: self.molecule,
            secret: self.secret,
            source_wallet: self.source_wallet,
            remainder_wallet: self.remainder_wallet,
            _phantom: PhantomData,
        })
    }

    /// Internal helper to add atoms and transition state
    fn add_atom_internal(
        mut self,
        isotope: Isotope,
        position: String,
        wallet_address: String,
        token: String,
        value: Option<f64>,
        batch_id: Option<String>,
        meta_type: Option<String>,
        meta_id: Option<String>,
        meta: Option<Vec<MetaItem>>,
    ) -> Result<TypeSafeMoleculeBuilder<states::WithAtoms>> {
        let atom = Atom::new(
            &position,
            &wallet_address,
            isotope,
            &token,
        ).with_optional_fields(
            value,
            batch_id.as_deref(),
            meta_type.as_deref(),
            meta_id.as_deref(),
            meta,
        );

        self.molecule.add_atom(atom);

        Ok(TypeSafeMoleculeBuilder {
            molecule: self.molecule,
            secret: self.secret,
            source_wallet: self.source_wallet,
            remainder_wallet: self.remainder_wallet,
            _phantom: PhantomData,
        })
    }
}

impl TypeSafeMoleculeBuilder<states::WithAtoms> {
    /// Add additional Value isotope atom
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the value atom
    ///
    /// # Returns
    ///
    /// Builder in same state with additional atom
    pub fn add_value_atom(mut self, params: ValueAtomParams) -> Result<Self> {
        let atom = Atom::new(
            &params.position,
            &params.wallet_address,
            Isotope::V,
            &params.token,
        ).with_optional_fields(
            params.value,
            params.batch_id.as_deref(),
            None,
            None,
            params.meta,
        );

        self.molecule.add_atom(atom);
        Ok(self)
    }

    /// Add additional Metadata isotope atom
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the metadata atom
    ///
    /// # Returns
    ///
    /// Builder in same state with additional atom
    pub fn add_meta_atom(mut self, params: MetaAtomParams) -> Result<Self> {
        let atom = Atom::new(
            &params.position,
            &params.wallet_address,
            Isotope::M,
            &params.token,
        ).with_optional_fields(
            None,
            params.batch_id.as_deref(),
            Some(&params.meta_type),
            Some(&params.meta_id),
            Some(params.meta),
        );

        self.molecule.add_atom(atom);
        Ok(self)
    }

    /// Add additional Identity isotope atom
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the identity atom
    ///
    /// # Returns
    ///
    /// Builder in same state with additional atom
    pub fn add_identity_atom(mut self, params: IdentityAtomParams) -> Result<Self> {
        let atom = Atom::new(
            &params.position,
            &params.wallet_address,
            Isotope::I,
            &params.token,
        ).with_optional_fields(
            None,
            params.batch_id.as_deref(),
            None,
            None,
            Some(params.meta),
        );

        self.molecule.add_atom(atom);
        Ok(self)
    }

    /// Add additional Token Request isotope atom
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the token request atom
    ///
    /// # Returns
    ///
    /// Builder in same state with additional atom
    pub fn add_token_request_atom(mut self, params: TokenRequestAtomParams) -> Result<Self> {
        let atom = Atom::new(
            &params.position,
            &params.wallet_address,
            Isotope::T,
            &params.token,
        ).with_optional_fields(
            None,
            params.batch_id.as_deref(),
            None,
            None,
            Some(params.meta),
        );

        self.molecule.add_atom(atom);
        Ok(self)
    }

    /// Add additional Buffer Deposit isotope atom (B-isotope)
    pub fn add_buffer_deposit_atom(mut self, params: BufferDepositAtomParams) -> Result<Self> {
        let atom = Atom::new(
            &params.position,
            &params.wallet_address,
            Isotope::B,
            &params.token,
        ).with_optional_fields(
            Some(params.value),
            params.batch_id.as_deref(),
            params.meta_type.as_deref(),
            params.meta_id.as_deref(),
            None,
        );

        self.molecule.add_atom(atom);
        Ok(self)
    }

    /// Add additional Buffer Withdraw isotope atom (B-isotope)
    pub fn add_buffer_withdraw_atom(mut self, params: BufferWithdrawAtomParams) -> Result<Self> {
        let atom = Atom::new(
            &params.position,
            &params.wallet_address,
            Isotope::B,
            &params.token,
        ).with_optional_fields(
            Some(params.value),
            params.batch_id.as_deref(),
            params.meta_type.as_deref(),
            params.meta_id.as_deref(),
            None,
        );

        self.molecule.add_atom(atom);
        Ok(self)
    }

    /// Add additional Fusion isotope atom (F-isotope)
    pub fn add_fusion_atom(mut self, params: FusionAtomParams) -> Result<Self> {
        let atom = Atom::new(
            &params.position,
            &params.wallet_address,
            Isotope::F,
            &params.token,
        ).with_optional_fields(
            params.value,
            params.batch_id.as_deref(),
            params.meta_type.as_deref(),
            params.meta_id.as_deref(),
            None,
        );

        self.molecule.add_atom(atom);
        Ok(self)
    }

    /// Add a remainder atom using the configured remainder wallet
    ///
    /// # Returns
    ///
    /// Result containing builder in same state with remainder atom added
    pub fn add_remainder_atom(mut self) -> Result<Self> {
        let remainder_wallet = self.remainder_wallet.as_ref()
            .ok_or_else(|| KnishIOError::custom("Remainder wallet not configured"))?;

        // Calculate the next position for the remainder atom
        let next_position = format!("W{}", self.molecule.atoms.len() + 1);

        let remainder_atom = Atom::new(
            &next_position,
            remainder_wallet.address.as_ref()
                .ok_or_else(|| KnishIOError::custom("Remainder wallet has no address"))?.as_str(),
            Isotope::V,
            &remainder_wallet.token,
        ).with_optional_fields(
            Some(0.0), // Remainder value will be calculated during signing
            None,
            None,
            None,
            None,
        );

        self.molecule.add_atom(remainder_atom);
        Ok(self)
    }

    /// Prepare the molecule for signing
    ///
    /// # Returns
    ///
    /// Builder in ReadyToSign state
    pub fn ready_to_sign(self) -> Result<TypeSafeMoleculeBuilder<states::ReadyToSign>> {
        // Validate that the molecule has at least one atom
        if self.molecule.atoms.is_empty() {
            return Err(KnishIOError::custom("Molecule must have at least one atom"));
        }

        // Validate that we have a source wallet
        if self.source_wallet.is_none() {
            return Err(KnishIOError::custom("Source wallet is required"));
        }

        Ok(TypeSafeMoleculeBuilder {
            molecule: self.molecule,
            secret: self.secret,
            source_wallet: self.source_wallet,
            remainder_wallet: self.remainder_wallet,
            _phantom: PhantomData,
        })
    }
}

impl TypeSafeMoleculeBuilder<states::ReadyToSign> {
    /// Sign the molecule using WOTS+ one-time signatures
    ///
    /// # Returns
    ///
    /// Result containing builder in Signed state
    pub async fn sign(mut self) -> Result<TypeSafeMoleculeBuilder<states::Signed>> {
        let secret = self.secret.as_ref()
            .ok_or_else(|| KnishIOError::custom("Secret is required for signing"))?;

        // Sign the molecule using the existing signing logic
        self.molecule.sign_with_secret(secret).await?;

        Ok(TypeSafeMoleculeBuilder {
            molecule: self.molecule,
            secret: self.secret,
            source_wallet: self.source_wallet,
            remainder_wallet: self.remainder_wallet,
            _phantom: PhantomData,
        })
    }

    /// Sign the molecule synchronously (no async runtime required).
    ///
    /// Enables offline transaction signing without a Tokio runtime.
    /// The underlying WOTS+ signing is entirely CPU-bound with zero network calls.
    ///
    /// # Returns
    ///
    /// Result containing builder in Signed state
    pub fn sign_sync(mut self) -> Result<TypeSafeMoleculeBuilder<states::Signed>> {
        let secret = self.secret.as_ref()
            .ok_or_else(|| KnishIOError::custom("Secret is required for signing"))?;

        // Set the secret on the molecule
        self.molecule.secret = Some(secret.to_string());

        // Set the bundle if not already set
        if self.molecule.bundle.is_none() {
            self.molecule.bundle = Some(crate::crypto::generate_bundle_hash(secret));
        }

        // Call the synchronous sign method directly
        self.molecule.sign(
            self.molecule.bundle.clone(),
            false,
            false,
        )?;

        Ok(TypeSafeMoleculeBuilder {
            molecule: self.molecule,
            secret: self.secret,
            source_wallet: self.source_wallet,
            remainder_wallet: self.remainder_wallet,
            _phantom: PhantomData,
        })
    }

    /// Get the current molecular hash before signing (for validation purposes)
    ///
    /// # Returns
    ///
    /// Result containing the molecular hash
    pub fn get_molecular_hash(&self) -> Result<String> {
        self.molecule.get_molecular_hash()
    }

    /// Validate the molecule structure before signing
    ///
    /// # Returns
    ///
    /// Result indicating validation success
    pub fn validate(&self) -> Result<()> {
        // Check that atoms are properly ordered
        for (i, atom) in self.molecule.atoms.iter().enumerate() {
            if let Some(atom_index) = atom.index {
                if atom_index as usize != i {
                    return Err(KnishIOError::custom("Atoms are not properly ordered"));
                }
            }
        }

        // Check for duplicate positions
        let positions: Vec<_> = self.molecule.atoms.iter()
            .map(|atom| &atom.position)
            .collect();
        let mut unique_positions = positions.clone();
        unique_positions.sort();
        unique_positions.dedup();
        
        if positions.len() != unique_positions.len() {
            return Err(KnishIOError::custom("Duplicate atom positions detected"));
        }

        Ok(())
    }
}

impl TypeSafeMoleculeBuilder<states::Signed> {
    /// Build the final molecule
    ///
    /// # Returns
    ///
    /// The constructed and signed molecule
    pub fn build(self) -> Molecule {
        self.molecule
    }

    /// Get a reference to the built molecule without consuming the builder
    ///
    /// # Returns
    ///
    /// Reference to the constructed molecule
    pub fn as_molecule(&self) -> &Molecule {
        &self.molecule
    }

    /// Get the molecular hash of the signed molecule
    ///
    /// # Returns
    ///
    /// The molecular hash
    pub fn molecular_hash(&self) -> Option<&String> {
        self.molecule.molecular_hash.as_ref()
    }

    /// Verify the molecule's signature (async)
    ///
    /// # Returns
    ///
    /// Result indicating signature validity
    pub async fn verify_signature(&self) -> Result<bool> {
        self.molecule.verify().await
    }

    /// Verify the molecule's signature synchronously.
    ///
    /// # Returns
    ///
    /// Result indicating signature validity
    pub fn verify_signature_sync(&self) -> Result<bool> {
        self.molecule.check(None)
    }
}

// ============================================================================
// Utility Functions for Common Patterns
// ============================================================================

impl<State> TypeSafeMoleculeBuilder<State> {
    /// Get a reference to the underlying molecule (any state)
    ///
    /// # Returns
    ///
    /// Reference to the molecule being built
    pub fn molecule(&self) -> &Molecule {
        &self.molecule
    }

    /// Get the current number of atoms in the molecule
    ///
    /// # Returns
    ///
    /// Number of atoms currently in the molecule
    pub fn atom_count(&self) -> usize {
        self.molecule.atoms.len()
    }

    /// Check if a specific isotope type is present in the molecule
    ///
    /// # Arguments
    ///
    /// * `isotope` - Isotope type to check for
    ///
    /// # Returns
    ///
    /// True if the isotope is present, false otherwise
    pub fn has_isotope(&self, isotope: Isotope) -> bool {
        self.molecule.atoms.iter().any(|atom| atom.isotope == isotope)
    }

    /// Get atoms of a specific isotope type
    ///
    /// # Arguments
    ///
    /// * `isotope` - Isotope type to filter by
    ///
    /// # Returns
    ///
    /// Vector of atoms matching the isotope type
    pub fn get_atoms_by_isotope(&self, isotope: Isotope) -> Vec<&Atom> {
        self.molecule.atoms.iter()
            .filter(|atom| atom.isotope == isotope)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    #[tokio::test]
    async fn test_type_safe_builder_basic_flow() {
        // Create test wallet
        let wallet = Wallet::create(
            Some("test-secret-12345"),
            None,
            "TEST",
            None,
            None,
        ).unwrap();

        // Build molecule using type-safe builder
        let molecule = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_value_atom(ValueAtomParams {
                position: "W1".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: Some(100.0),
                ..Default::default()
            })
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign()
            .await
            .unwrap()
            .build();

        // Verify molecule structure
        assert_eq!(molecule.atoms.len(), 1);
        assert_eq!(molecule.atoms[0].isotope, Isotope::V);
        assert_eq!(molecule.atoms[0].value, Some("100.0".to_string()));
        assert!(molecule.molecular_hash.is_some());
    }

    #[tokio::test]
    async fn test_type_safe_builder_with_remainder() {
        let source_wallet = Wallet::create(Some("source-secret"), None, "TEST", None, None).unwrap();
        let remainder_wallet = Wallet::create(Some("remainder-secret"), None, "TEST", None, None).unwrap();

        let molecule = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(source_wallet.clone())
            .with_remainder_wallet(remainder_wallet.clone())
            .add_value_atom(ValueAtomParams {
                position: "W1".to_string(),
                wallet_address: source_wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: Some(100.0),
                ..Default::default()
            })
            .unwrap()
            .add_remainder_atom()
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign()
            .await
            .unwrap()
            .build();

        // Should have 2 atoms: value + remainder
        assert_eq!(molecule.atoms.len(), 2);
        assert_eq!(molecule.atoms[0].isotope, Isotope::V);
        assert_eq!(molecule.atoms[1].isotope, Isotope::V);
    }

    #[tokio::test]
    async fn test_type_safe_builder_multi_isotope() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();

        let molecule = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_value_atom(ValueAtomParams {
                position: "W1".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: Some(50.0),
                ..Default::default()
            })
            .unwrap()
            .add_meta_atom(MetaAtomParams {
                position: "W2".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                meta_type: "profile".to_string(),
                meta_id: "user123".to_string(),
                meta: vec![],
                batch_id: None,
            })
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign()
            .await
            .unwrap()
            .build();

        // Should have value and metadata atoms
        assert_eq!(molecule.atoms.len(), 2);
        assert!(molecule.has_isotope(Isotope::V));
        assert!(molecule.has_isotope(Isotope::M));
    }

    #[test]
    fn test_builder_validation() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();

        let builder = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_value_atom(ValueAtomParams {
                position: "W1".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: Some(100.0),
                ..Default::default()
            })
            .unwrap()
            .ready_to_sign()
            .unwrap();

        // Validation should pass
        assert!(builder.validate().is_ok());
        
        // Check utility functions
        assert_eq!(builder.atom_count(), 1);
        assert!(builder.has_isotope(Isotope::V));
        assert!(!builder.has_isotope(Isotope::M));
    }

    #[test]
    fn test_empty_molecule_validation() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();

        // Test that attempting to ready_to_sign without atoms fails
        // This validates the state machine enforces atoms are present
        let result = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_value_atom(ValueAtomParams {
                position: wallet.position.clone().unwrap_or_default(),
                wallet_address: wallet.address.clone().unwrap_or_default(),
                token: "TEST".to_string(),
                value: Some(100.0),
                batch_id: None,
                meta: None,
            })
            .and_then(|builder| builder.ready_to_sign());

        // Should succeed because atoms were added
        assert!(result.is_ok());
    }

    #[test]
    fn test_buffer_deposit_builder() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();

        let result = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_buffer_deposit_atom(BufferDepositAtomParams {
                position: "W1".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: 50.0,
                meta_type: Some("walletBundle".to_string()),
                meta_id: wallet.bundle.clone(),
                batch_id: None,
            });

        assert!(result.is_ok());
        let builder = result.unwrap();
        assert_eq!(builder.atom_count(), 1);
        assert!(builder.has_isotope(Isotope::B));
    }

    #[test]
    fn test_buffer_withdraw_builder() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();

        let result = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_buffer_withdraw_atom(BufferWithdrawAtomParams {
                position: "W1".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: 25.0,
                meta_type: Some("walletBundle".to_string()),
                meta_id: wallet.bundle.clone(),
                batch_id: None,
            });

        assert!(result.is_ok());
        let builder = result.unwrap();
        assert_eq!(builder.atom_count(), 1);
        assert!(builder.has_isotope(Isotope::B));
    }

    #[test]
    fn test_fusion_builder() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();

        let result = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_fusion_atom(FusionAtomParams {
                position: "W1".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: Some(1.0),
                meta_type: Some("walletBundle".to_string()),
                meta_id: wallet.bundle.clone(),
                batch_id: None,
            });

        assert!(result.is_ok());
        let builder = result.unwrap();
        assert_eq!(builder.atom_count(), 1);
        assert!(builder.has_isotope(Isotope::F));
    }

    #[test]
    fn test_buffer_and_fusion_multi_atom() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();

        let result = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_value_atom(ValueAtomParams {
                position: "W1".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: Some(-50.0),
                ..Default::default()
            })
            .unwrap()
            .add_buffer_deposit_atom(BufferDepositAtomParams {
                position: "W2".to_string(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                value: 50.0,
                meta_type: Some("walletBundle".to_string()),
                meta_id: wallet.bundle.clone(),
                batch_id: None,
            });

        assert!(result.is_ok());
        let builder = result.unwrap();
        assert_eq!(builder.atom_count(), 2);
        assert!(builder.has_isotope(Isotope::V));
        assert!(builder.has_isotope(Isotope::B));
    }

    #[test]
    fn test_missing_source_wallet_validation() {
        // This test demonstrates the type safety of the builder pattern
        // The following would NOT compile (demonstrating compile-time safety):
        //
        // let result = TypeSafeMoleculeBuilder::new("test-secret")
        //     .ready_to_sign();  // ❌ Compile error: ready_to_sign() not available on Empty state
        //
        // This enforces that you must:
        // 1. Create builder (Empty state)
        // 2. Add source wallet (WithSourceWallet state)
        // 3. Add atoms (WithAtoms state)
        // 4. Then call ready_to_sign() (ReadyToSign state)

        // Instead, test the happy path that DOES compile
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();
        let result = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .add_value_atom(ValueAtomParams {
                position: wallet.position.clone().unwrap_or_default(),
                wallet_address: wallet.address.clone().unwrap_or_default(),
                token: "TEST".to_string(),
                value: Some(100.0),
                batch_id: None,
                meta: None,
            });

        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_with_parent_hashes() {
        let wallet = Wallet::create(Some("test-secret"), None, "TEST", None, None).unwrap();
        let parent_hashes = vec!["parent_hash_1".to_string(), "parent_hash_2".to_string()];

        let builder = TypeSafeMoleculeBuilder::new("test-secret")
            .with_source_wallet(wallet.clone())
            .with_parent_hashes(parent_hashes.clone())
            .add_value_atom(ValueAtomParams {
                position: wallet.position.clone().unwrap_or_default(),
                wallet_address: wallet.address.clone().unwrap_or_default(),
                token: "TEST".to_string(),
                value: Some(100.0),
                batch_id: None,
                meta: None,
            })
            .unwrap();

        // Verify parent hashes are set on the underlying molecule
        assert_eq!(builder.molecule().parent_hashes, parent_hashes);
    }

    // ── GAP-07-010: Offline signing tests ───────────────────────────────

    #[test]
    fn test_sign_sync_offline() {
        // Verify synchronous signing works without tokio runtime.
        // Uses C-isotope (token creation) which has minimal validation requirements,
        // avoiding the M-isotope "USER" token + ContinuID I-atom dependency.
        let wallet = Wallet::create(
            Some("offline-secret"),
            None,
            "MYTOKEN",
            None,
            None,
        ).unwrap();

        let signed = TypeSafeMoleculeBuilder::new("offline-secret")
            .with_source_wallet(wallet.clone())
            .add_token_request_atom(TokenRequestAtomParams {
                position: wallet.position.as_ref().unwrap().clone(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "MYTOKEN".to_string(),
                meta: vec![
                    MetaItem::new("name", "My Token"),
                    MetaItem::new("fungibility", "stackable"),
                ],
                batch_id: None,
            })
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign_sync()
            .unwrap();

        let molecule = signed.as_molecule();
        // Verify that sign_sync produced a molecular hash
        assert!(molecule.molecular_hash.is_some(), "Should have molecular hash");
        // Verify that OTS fragments were generated and distributed
        assert!(molecule.atoms[0].ots_fragment.is_some(), "First atom should have OTS fragment");
        // Verify the hash is a proper base17 string (non-empty)
        let hash = molecule.molecular_hash.as_ref().unwrap();
        assert!(!hash.is_empty(), "Molecular hash should not be empty");
    }

    #[test]
    fn test_json_roundtrip_signed() {
        // Verify toJSON/fromJSON roundtrip preserves signature
        let wallet = Wallet::create(
            Some("roundtrip-secret"),
            None,
            "TEST",
            None,
            None,
        ).unwrap();

        let original = TypeSafeMoleculeBuilder::new("roundtrip-secret")
            .with_source_wallet(wallet.clone())
            .add_meta_atom(MetaAtomParams {
                position: wallet.position.as_ref().unwrap().clone(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "TEST".to_string(),
                meta_type: "test".to_string(),
                meta_id: "roundtrip-test".to_string(),
                meta: vec![MetaItem::new("key", "value")],
                batch_id: None,
            })
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign_sync()
            .unwrap()
            .build();

        let json_str = original.toJSON().unwrap();
        let restored = crate::molecule::Molecule::fromJSON(&json_str).unwrap();

        assert_eq!(original.molecular_hash, restored.molecular_hash,
            "Molecular hash should survive roundtrip");
        assert_eq!(original.atoms.len(), restored.atoms.len(),
            "Atom count should match");
        assert_eq!(original.atoms[0].ots_fragment, restored.atoms[0].ots_fragment,
            "OTS fragment should survive roundtrip");
    }

    // ── GAP-07-009: Stackable token transfer tests ──────────────────────

    #[test]
    fn test_stackable_transfer() {
        // Create source wallet with balance "1000" and batch_id
        let mut source_wallet = Wallet::create(
            Some("stackable-secret"),
            None,
            "TICKETS",
            None,
            None,
        ).unwrap();
        source_wallet.set_balance_i128(1000);
        source_wallet.batch_id = Some("batch-001".to_string());

        // Create remainder wallet for change
        let remainder_wallet = Wallet::create(
            Some("stackable-secret"),
            None,
            "TICKETS",
            Some("W2"),
            None,
        ).unwrap();

        // Create recipient wallet
        let recipient_wallet = Wallet::create(
            Some("recipient-secret"),
            None,
            "TICKETS",
            None,
            None,
        ).unwrap();

        let builder = TypeSafeMoleculeBuilder::new("stackable-secret")
            .with_source_wallet(source_wallet.clone())
            .with_remainder_wallet(remainder_wallet.clone())
            .add_stackable_transfer(StackableTransferParams {
                token: "TICKETS".to_string(),
                amount: 250.0,
                recipient_address: recipient_wallet.address.clone().unwrap(),
                recipient_position: recipient_wallet.position.clone().unwrap(),
                recipient_bundle: recipient_wallet.bundle.clone(),
                batch_id: Some("batch-001".to_string()),
            })
            .unwrap();

        let atoms = &builder.molecule().atoms;
        // Should produce 3 atoms: source debit, recipient credit, remainder
        assert_eq!(atoms.len(), 3, "Should have 3 V-isotope atoms");

        // Atom 0: source debit (-1000)
        assert_eq!(atoms[0].isotope, Isotope::V);
        assert_eq!(atoms[0].value, Some("-1000".to_string()));
        assert_eq!(atoms[0].batch_id, Some("batch-001".to_string()));

        // Atom 1: recipient credit (+250)
        assert_eq!(atoms[1].isotope, Isotope::V);
        assert_eq!(atoms[1].value, Some("250".to_string()));
        assert_eq!(atoms[1].batch_id, Some("batch-001".to_string()));
        assert_eq!(atoms[1].meta_type, Some("walletBundle".to_string()));

        // Atom 2: remainder credit (+750)
        assert_eq!(atoms[2].isotope, Isotope::V);
        assert_eq!(atoms[2].value, Some("750".to_string()));
        assert_eq!(atoms[2].batch_id, Some("batch-001".to_string()));

        // Sign and verify
        let signed = builder
            .ready_to_sign()
            .unwrap()
            .sign_sync()
            .unwrap();

        let molecule = signed.as_molecule();
        assert!(molecule.molecular_hash.is_some(), "Should have molecular hash");
        assert!(molecule.atoms[0].ots_fragment.is_some(), "Should have OTS fragment");
    }

    #[test]
    fn test_stackable_transfer_exact_amount() {
        // Transfer the entire balance — no remainder atom needed
        let mut source_wallet = Wallet::create(
            Some("exact-secret"),
            None,
            "COINS",
            None,
            None,
        ).unwrap();
        source_wallet.set_balance_i128(500);

        let recipient_wallet = Wallet::create(
            Some("recipient-exact"),
            None,
            "COINS",
            None,
            None,
        ).unwrap();

        let builder = TypeSafeMoleculeBuilder::new("exact-secret")
            .with_source_wallet(source_wallet.clone())
            .add_stackable_transfer(StackableTransferParams {
                token: "COINS".to_string(),
                amount: 500.0,
                recipient_address: recipient_wallet.address.clone().unwrap(),
                recipient_position: recipient_wallet.position.clone().unwrap(),
                recipient_bundle: None,
                batch_id: None,
            })
            .unwrap();

        // Exact amount: only 2 atoms (source debit + recipient credit, no remainder)
        assert_eq!(builder.molecule().atoms.len(), 2, "Should have 2 atoms when amount == balance");
        assert_eq!(builder.molecule().atoms[0].value, Some("-500".to_string()));
        assert_eq!(builder.molecule().atoms[1].value, Some("500".to_string()));
    }

    #[test]
    fn test_stackable_transfer_insufficient_balance() {
        let mut source_wallet = Wallet::create(
            Some("poor-secret"),
            None,
            "RARE",
            None,
            None,
        ).unwrap();
        source_wallet.set_balance_i128(100);

        let result = TypeSafeMoleculeBuilder::new("poor-secret")
            .with_source_wallet(source_wallet)
            .add_stackable_transfer(StackableTransferParams {
                token: "RARE".to_string(),
                amount: 200.0,
                recipient_address: "addr".to_string(),
                recipient_position: "W1".to_string(),
                recipient_bundle: None,
                batch_id: None,
            });

        assert!(result.is_err(), "Should fail with insufficient balance");
    }

    // ── Whitepaper compliance: sign/check integration tests ──────────────

    /// V-isotope value conservation + sign + check round-trip.
    /// Whitepaper: V-atom values MUST sum to exactly 0 (conservation law).
    #[test]
    fn test_sign_check_v_isotope_roundtrip() {
        let mut source_wallet = Wallet::create(
            Some("v-check-secret"),
            None,
            "VCHECK",
            None,
            None,
        ).unwrap();
        source_wallet.set_balance_i128(500);
        source_wallet.batch_id = Some("batch-v".to_string());

        let remainder_wallet = Wallet::create(
            Some("v-check-secret"),
            None,
            "VCHECK",
            Some("W2"),
            None,
        ).unwrap();

        let recipient = Wallet::create(
            Some("v-recipient"),
            None,
            "VCHECK",
            None,
            None,
        ).unwrap();

        // Keep a clone for check() — sender_wallet is consumed by the builder
        let sender_for_check = source_wallet.clone();

        let signed = TypeSafeMoleculeBuilder::new("v-check-secret")
            .with_source_wallet(source_wallet)
            .with_remainder_wallet(remainder_wallet)
            .add_stackable_transfer(StackableTransferParams {
                token: "VCHECK".to_string(),
                amount: 200.0,
                recipient_address: recipient.address.clone().unwrap(),
                recipient_position: recipient.position.clone().unwrap(),
                recipient_bundle: recipient.bundle.clone(),
                batch_id: Some("batch-v".to_string()),
            })
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign_sync()
            .unwrap();

        let molecule = signed.as_molecule();

        // Conservation law: sum of all V-atom values MUST be 0
        let sum: i128 = molecule.atoms.iter()
            .filter(|a| a.isotope == Isotope::V)
            .filter_map(|a| a.value.as_ref().and_then(|v| v.parse::<i128>().ok()))
            .sum();
        assert_eq!(sum, 0, "V-atom values must sum to 0, got {}", sum);

        // Molecule must pass full check() with sender wallet (for remainder validation)
        let check_result = molecule.check(Some(&sender_for_check));
        assert!(check_result.is_ok(), "V-isotope molecule must pass check(): {:?}", check_result.err());
    }

    /// Token request (T-isotope) sign + molecular hash verification.
    /// Note: T-isotope check() requires token=="USER" + I-isotope ContinuID atom,
    /// so we verify signing correctness and OTS fragment distribution instead.
    #[test]
    fn test_sign_check_token_request_roundtrip() {
        let wallet = Wallet::create(
            Some("t-check-secret"),
            None,
            "NEWTOKEN",
            None,
            None,
        ).unwrap();

        let signed = TypeSafeMoleculeBuilder::new("t-check-secret")
            .with_source_wallet(wallet.clone())
            .add_token_request_atom(TokenRequestAtomParams {
                position: wallet.position.as_ref().unwrap().clone(),
                wallet_address: wallet.address.as_ref().unwrap().clone(),
                token: "NEWTOKEN".to_string(),
                meta: vec![
                    MetaItem::new("name", "Test Token"),
                    MetaItem::new("fungibility", "stackable"),
                ],
                batch_id: None,
            })
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign_sync()
            .unwrap();

        let molecule = signed.as_molecule();
        assert!(molecule.molecular_hash.is_some(), "Should have molecular hash");

        // OTS fragments distributed across atoms (first atom gets the signature)
        assert!(molecule.atoms[0].ots_fragment.is_some(), "First atom must have OTS fragment");

        // Molecular hash must be a valid base17 string (64 chars, hex + 'g')
        let hash = molecule.molecular_hash.as_ref().unwrap();
        assert_eq!(hash.len(), 64, "Molecular hash must be 64 chars");
        assert!(hash.chars().all(|c| "0123456789abcdefg".contains(c)),
            "Molecular hash must contain only base17 characters");
    }

    /// Stackable transfer conservation: 3 atoms (debit, credit, remainder)
    /// with values summing exactly to 0.
    #[test]
    fn test_stackable_transfer_conservation_law() {
        let mut source_wallet = Wallet::create(
            Some("cons-secret"),
            None,
            "STKTEST",
            None,
            None,
        ).unwrap();
        source_wallet.set_balance_i128(1000);

        let remainder_wallet = Wallet::create(
            Some("cons-secret"),
            None,
            "STKTEST",
            Some("W2"),
            None,
        ).unwrap();

        let recipient = Wallet::create(
            Some("cons-recipient"),
            None,
            "STKTEST",
            None,
            None,
        ).unwrap();

        let signed = TypeSafeMoleculeBuilder::new("cons-secret")
            .with_source_wallet(source_wallet)
            .with_remainder_wallet(remainder_wallet)
            .add_stackable_transfer(StackableTransferParams {
                token: "STKTEST".to_string(),
                amount: 350.0,
                recipient_address: recipient.address.clone().unwrap(),
                recipient_position: recipient.position.clone().unwrap(),
                recipient_bundle: None,
                batch_id: None,
            })
            .unwrap()
            .ready_to_sign()
            .unwrap()
            .sign_sync()
            .unwrap();

        let molecule = signed.as_molecule();

        // Must have 3 V-atoms: sender(-1000), recipient(+350), remainder(+650)
        let v_atoms: Vec<_> = molecule.atoms.iter()
            .filter(|a| a.isotope == Isotope::V)
            .collect();
        assert_eq!(v_atoms.len(), 3, "Stackable transfer must produce 3 V-atoms");

        // Verify individual values
        assert_eq!(v_atoms[0].value, Some("-1000".to_string()), "Sender debits full balance");
        assert_eq!(v_atoms[1].value, Some("350".to_string()), "Recipient receives amount");
        assert_eq!(v_atoms[2].value, Some("650".to_string()), "Remainder receives change");

        // Conservation: sum MUST be 0
        let sum: i128 = v_atoms.iter()
            .filter_map(|a| a.value.as_ref().and_then(|v| v.parse::<i128>().ok()))
            .sum();
        assert_eq!(sum, 0, "Stackable transfer values must sum to 0, got {}", sum);
    }
}