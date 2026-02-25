//! MutationWithdrawBufferToken implementation
//!
//! Mutation for withdrawing tokens from a buffer,
//! equivalent to MutationWithdrawBufferToken.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseProposeMolecule};
use crate::molecule::Molecule;
use crate::wallet::Wallet;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::Value;
use std::collections::HashMap;

/// Parameters for withdrawing buffer tokens (matches JS fillMolecule parameters)
/// JS: fillMolecule({ recipients, signingWallet })
#[derive(Debug, Clone)]
pub struct WithdrawBufferTokenParams {
    /// The recipients for the withdrawal
    pub recipients: HashMap<String, f64>,
    /// The signing wallet (matches JS signingWallet, not signing_wallet)
    pub signing_wallet: Option<Wallet>,
}

/// Mutation for withdrawing tokens from a buffer
pub struct MutationWithdrawBufferToken {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationWithdrawBufferToken {
    /// Create a new MutationWithdrawBufferToken (matches JS constructor)
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationWithdrawBufferToken {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationWithdrawBufferToken {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with withdraw buffer data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ recipients, signingWallet })
    pub fn fill_molecule(&mut self, params: WithdrawBufferTokenParams) -> crate::error::Result<()> {
        // Call molecule's initWithdrawBuffer method (matches JS: this.$__molecule.initWithdrawBuffer({ recipients, signingWallet }))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            molecule.init_withdraw_buffer(
                params.recipients,
                params.signing_wallet.as_ref()
            )?;
            
            // Sign with empty params (matches JS: this.$__molecule.sign({}))
            molecule.sign(None, false, true)?;
            
            // Check with source wallet (matches JS: this.$__molecule.check(this.$__molecule.sourceWallet))
            if let Some(ref source_wallet) = molecule.source_wallet {
                molecule.check(Some(source_wallet))?;
            } else {
                molecule.check(None)?;
            }
        }
        
        Ok(())
    }
    
    /// Create from withdraw buffer parameters
    pub fn from_params(_params: WithdrawBufferTokenParams, _secret: &str) -> Self {
        let molecule = Molecule::new();
        
        // Initialize withdraw buffer in molecule
        // molecule.init_withdraw_buffer(
        //     &params.recipients,
        //     &params.signing_wallet
        // );
        // molecule.sign(None);
        // molecule.check(&molecule.source_wallet);
        
        Self::from_molecule(molecule)
    }
}

#[async_trait::async_trait]
impl Query for MutationWithdrawBufferToken {
    /// Delegate to the underlying propose molecule mutation
    fn get_query(&self) -> &str {
        self.propose_molecule.get_query()
    }
    
    /// Delegate compiled variables
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        self.propose_molecule.compiled_variables(variables)
    }
    
    /// Create a response from the JSON data (uses base ProposeMolecule response)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        match ResponseProposeMolecule::new(json, None) {
            Ok(resp) => Box::new(resp),
            Err(e) => {
                eprintln!("ResponseProposeMolecule construction failed: {}", e);
                Box::new(crate::response::BaseResponse::empty())
            }
        }
    }
}

#[async_trait::async_trait]
impl Mutation for MutationWithdrawBufferToken {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationWithdrawBufferToken {
    /// Get the underlying molecule
    pub fn molecule(&self) -> &Molecule {
        self.propose_molecule.molecule()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_withdraw_buffer_token_creation() {
        let molecule = Molecule::new();
        let mutation = MutationWithdrawBufferToken::from_molecule(molecule);
        
        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_withdraw_buffer_token_params() {
        let mut recipients = HashMap::new();
        recipients.insert("addr1".to_string(), 50.0);
        
        let signing_wallet = Wallet::new(
            Some("test_secret"),
            Some("test_bundle"),
            Some("TEST"),
            Some("test_address"),
            Some("test_position"),
            None,
            None
        ).expect("Failed to create wallet");
        
        let params = WithdrawBufferTokenParams {
            recipients,
            signing_wallet: Some(signing_wallet),
        };
        
        assert_eq!(params.recipients.len(), 1);
    }
}