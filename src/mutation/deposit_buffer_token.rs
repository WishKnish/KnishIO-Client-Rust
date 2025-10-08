//! MutationDepositBufferToken implementation
//!
//! Mutation for depositing tokens to a buffer,
//! equivalent to MutationDepositBufferToken.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseProposeMolecule};
use crate::molecule::Molecule;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::Value;
use std::collections::HashMap;

/// Parameters for depositing buffer tokens
#[derive(Debug, Clone)]
pub struct DepositBufferTokenParams {
    /// The amount to deposit
    pub amount: f64,
    /// Trade rates information
    pub trade_rates: HashMap<String, f64>,
}

/// Mutation for depositing tokens to a buffer
pub struct MutationDepositBufferToken {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationDepositBufferToken {
    /// Create a new MutationDepositBufferToken (matches JS constructor)
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationDepositBufferToken {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationDepositBufferToken {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with deposit buffer data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ amount, tradeRates })
    pub fn fill_molecule(&mut self, params: DepositBufferTokenParams) -> crate::error::Result<()> {
        // Call molecule's initDepositBuffer method (matches JS: this.$__molecule.initDepositBuffer({ amount, tradeRates }))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            molecule.init_deposit_buffer(
                params.amount,
                params.trade_rates
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
    
    /// Create from deposit buffer parameters
    pub fn from_params(_params: DepositBufferTokenParams, _secret: &str) -> Self {
        let molecule = Molecule::new();
        
        // Initialize deposit buffer in molecule
        // molecule.init_deposit_buffer(
        //     &params.amount,
        //     &params.trade_rates
        // );
        // molecule.sign(None);
        // molecule.check(&molecule.source_wallet);
        
        Self::from_molecule(molecule)
    }
}

#[async_trait::async_trait]
impl Query for MutationDepositBufferToken {
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
        Box::new(ResponseProposeMolecule::new(json, None).expect("Failed to create ResponseProposeMolecule"))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationDepositBufferToken {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationDepositBufferToken {
    /// Get the underlying molecule
    pub fn molecule(&self) -> &Molecule {
        self.propose_molecule.molecule()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_deposit_buffer_token_creation() {
        let molecule = Molecule::new();
        let mutation = MutationDepositBufferToken::from_molecule(molecule);
        
        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_deposit_buffer_token_params() {
        let mut trade_rates = HashMap::new();
        trade_rates.insert("USD".to_string(), 1.0);
        trade_rates.insert("EUR".to_string(), 0.85);
        
        let params = DepositBufferTokenParams {
            amount: 100.0,
            trade_rates,
        };
        
        assert_eq!(params.amount, 100.0);
        assert_eq!(params.trade_rates.len(), 2);
    }
}