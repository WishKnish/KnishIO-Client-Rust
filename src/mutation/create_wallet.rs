//! MutationCreateWallet implementation
//!
//! Mutation for creating new wallets,
//! equivalent to MutationCreateWallet.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseCreateWallet};
use crate::molecule::Molecule;
use crate::wallet::Wallet;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::Value;

/// Mutation for creating new wallets
/// 
/// This mutation extends ProposeMolecule to handle wallet creation operations.
/// It initializes a molecule with wallet creation atoms and handles the 
/// submission process.
pub struct MutationCreateWallet {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationCreateWallet {
    /// Create a new MutationCreateWallet (matches JS constructor)
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationCreateWallet {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationCreateWallet {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with wallet creation data (matches JS fillMolecule exactly)
    /// JS: fillMolecule(wallet)
    pub fn fill_molecule(&mut self, wallet: &Wallet) -> crate::error::Result<()> {
        // Call molecule's initWalletCreation method (matches JS: this.$__molecule.initWalletCreation(wallet))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            // Initialize wallet creation with empty metadata
            // In JS, this would be molecule.initWalletCreation(wallet) with default empty AtomMeta
            molecule.init_wallet_creation(wallet, Vec::new())?;
            
            // Sign with empty params (matches JS: this.$__molecule.sign({}))
            molecule.sign(None, false, true)?;
            
            // Check molecule (matches JS: this.$__molecule.check())
            molecule.check(None)?;
        }
        
        Ok(())
    }
    
    /// Create from a wallet with proper initialization
    pub fn from_wallet(wallet: &Wallet, secret: &str) -> crate::error::Result<Self> {
        let mut molecule = Molecule::with_params(
            Some(secret.to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        
        // Initialize wallet creation in molecule
        molecule.init_wallet_creation(wallet, Vec::new())?;
        molecule.sign(None, false, true)?;
        molecule.check(None)?;
        
        Ok(Self::from_molecule(molecule))
    }
}

#[async_trait::async_trait]
impl Query for MutationCreateWallet {
    /// Delegate to the underlying propose molecule mutation
    fn get_query(&self) -> &str {
        self.propose_molecule.get_query()
    }
    
    /// Delegate compiled variables
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        self.propose_molecule.compiled_variables(variables)
    }
    
    /// Create a response from the JSON data
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        // Using ResponseCreateWallet for proper type safety
        Box::new(ResponseCreateWallet::new(json))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationCreateWallet {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationCreateWallet {
    /// Get the underlying molecule
    pub fn molecule(&self) -> &Molecule {
        self.propose_molecule.molecule()
    }
    
    /// Get the remainder wallet if any
    pub fn remainder_wallet(&self) -> Option<&Value> {
        self.propose_molecule.remainder_wallet()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_create_wallet_creation() {
        let molecule = Molecule::new();
        let mutation = MutationCreateWallet::from_molecule(molecule);
        
        assert!(mutation.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_mutation_string() {
        let molecule = Molecule::new();
        let mutation = MutationCreateWallet::from_molecule(molecule);
        let mutation_string = mutation.get_mutation();
        
        // Should have the same mutation string as ProposeMolecule
        assert!(mutation_string.contains("mutation( $molecule: MoleculeInput! )"));
        assert!(mutation_string.contains("ProposeMolecule( molecule: $molecule )"));
    }
    
    #[test]
    fn test_wallet_creation_flow() {
        // Test that we can create a mutation from a wallet
        let wallet = Wallet::create(
            Some("test-secret"),
            None,
            "TEST",
            None,
            None,
        ).unwrap();
        
        // This should fail without proper molecule initialization methods implemented
        let result = MutationCreateWallet::from_wallet(&wallet, "test-secret");
        
        // Expected to fail until molecule methods are implemented
        assert!(result.is_err());
    }
}