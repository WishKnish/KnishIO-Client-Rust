//! MutationTransferTokens implementation
//!
//! Mutation for moving tokens between wallets,
//! equivalent to MutationTransferTokens.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseTransferTokens};
use crate::molecule::Molecule;
use crate::wallet::Wallet;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::Value;

/// Parameters for fillMolecule (matches JS MutationTransferTokens fillMolecule parameters)
/// JS: fillMolecule({ recipientWallet, amount })
#[derive(Debug, Clone)]
pub struct TransferTokensParams {
    /// The recipient wallet
    pub recipient_wallet: Wallet,
    /// The amount to transfer
    pub amount: f64,
}

/// Mutation for moving tokens between wallets
pub struct MutationTransferTokens {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationTransferTokens {
    /// Create a new MutationTransferTokens (matches JS constructor)
    /// Takes graphQLClient, knishIOClient, and molecule like JS constructor
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationTransferTokens {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility and simpler usage)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationTransferTokens {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with transfer data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ recipientWallet, amount })
    pub fn fill_molecule(&mut self, params: TransferTokensParams) -> crate::error::Result<()> {
        // Call molecule's initValue method (matches JS: this.$__molecule.initValue({recipientWallet, amount}))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            molecule.init_value(&params.recipient_wallet, params.amount)?;
            
            // Sign with empty params (matches JS: this.$__molecule.sign({}))
            molecule.sign(
                None, // empty bundle for sign({})
                false, // anonymous = false
                true   // compressed = true
            )?;
            
            // Check with source wallet (matches JS: this.$__molecule.check(this.$__molecule.sourceWallet))
            if let Some(ref source_wallet) = molecule.source_wallet {
                molecule.check(Some(source_wallet))?;
            } else {
                molecule.check(None)?;
            }
        }
        
        Ok(())
    }
    
}

#[async_trait::async_trait]
impl Query for MutationTransferTokens {
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
        // Using ResponseTransferTokens for proper type safety
        Box::new(ResponseTransferTokens::new(json))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationTransferTokens {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationTransferTokens {
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
    fn test_mutation_transfer_tokens_creation() {
        let molecule = Molecule::new();
        let mutation = MutationTransferTokens::from_molecule(molecule);
        
        assert!(mutation.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_mutation_string() {
        let molecule = Molecule::new();
        let mutation = MutationTransferTokens::from_molecule(molecule);
        let mutation_string = mutation.get_mutation();
        
        // Should have the same mutation string as ProposeMolecule
        assert!(mutation_string.contains("mutation( $molecule: MoleculeInput! )"));
        assert!(mutation_string.contains("ProposeMolecule( molecule: $molecule )"));
    }
    
    #[test]
    fn test_transfer_params() {
        let params = TransferTokensParams {
            recipient_wallet: Wallet::new(Some("recipient-secret"), None, Some("TEST"), None, None, None, None).unwrap(),
            amount: 50.0,
        };
        
        assert_eq!(params.amount, 50.0);
    }
}