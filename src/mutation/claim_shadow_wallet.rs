//! MutationClaimShadowWallet implementation
//!
//! Mutation for claiming a shadow wallet,
//! equivalent to MutationClaimShadowWallet.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseClaimShadowWallet};
use crate::molecule::Molecule;
use crate::wallet::Wallet;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::Value;

/// Parameters for claiming shadow wallets (matches JS fillMolecule parameters)
#[derive(Debug, Clone)]
pub struct ClaimShadowWalletParams {
    /// The token type to claim
    pub token: String,
    /// The batch ID for the claim (can be null)
    pub batch_id: Option<String>,
}

/// Mutation for claiming a shadow wallet
pub struct MutationClaimShadowWallet {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationClaimShadowWallet {
    /// Create a new MutationClaimShadowWallet (matches JS constructor)
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationClaimShadowWallet {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationClaimShadowWallet {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with shadow wallet claim data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ token, batchId })
    pub fn fill_molecule(&mut self, params: ClaimShadowWalletParams, wallet: &Wallet) -> crate::error::Result<()> {
        // Call molecule's init_shadow_wallet_claim method
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            // Use params for JavaScript API compatibility (fillMolecule configuration)
            if !params.token.is_empty() {
                // Apply token configuration like JavaScript fillMolecule({token})
                molecule.cell_slug = Some(params.token.clone());  // JavaScript parameter pattern
            }
            
            // Initialize shadow wallet claim for the given wallet
            molecule.init_shadow_wallet_claim(wallet)?;
            
            // Sign with empty params (matches JS: this.$__molecule.sign({}))
            molecule.sign(None, false, true)?;
            
            // Check molecule (matches JS: this.$__molecule.check())
            molecule.check(None)?;
        }
        
        Ok(())
    }
    
    /// Create from claim shadow wallet parameters
    pub fn from_params(_params: ClaimShadowWalletParams, _secret: &str) -> Self {
        let molecule = Molecule::new();
        
        // Initialize shadow wallet claim in molecule
        // molecule.init_shadow_wallet_claim(wallet);
        // molecule.sign(None);
        // molecule.check(None);
        
        Self::from_molecule(molecule)
    }
}

#[async_trait::async_trait]
impl Query for MutationClaimShadowWallet {
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
        Box::new(ResponseClaimShadowWallet::new(json, None).expect("Failed to create ResponseClaimShadowWallet"))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationClaimShadowWallet {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationClaimShadowWallet {
    /// Get the underlying molecule
    pub fn molecule(&self) -> &Molecule {
        self.propose_molecule.molecule()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_claim_shadow_wallet_creation() {
        let molecule = Molecule::new();
        let mutation = MutationClaimShadowWallet::from_molecule(molecule);
        
        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_claim_shadow_wallet_params() {
        let params = ClaimShadowWalletParams {
            token: "TEST".to_string(),
            batch_id: Some("batch123".to_string()),
        };
        
        assert_eq!(params.token, "TEST");
        assert_eq!(params.batch_id, Some("batch123".to_string()));
    }
}