//! MutationCreateToken implementation
//!
//! Mutation for creating new tokens,
//! equivalent to MutationCreateToken.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseCreateToken};
use crate::molecule::Molecule;
use crate::wallet::Wallet;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use crate::types::MetaItem;
use serde_json::Value;
use std::collections::HashMap;

/// Parameters for fillMolecule (matches JS MutationCreateToken fillMolecule parameters)
/// JS: fillMolecule({ recipientWallet, amount, meta = null })
#[derive(Debug, Clone)]
pub struct CreateTokenParams {
    /// Recipient wallet receiving the tokens
    pub recipient_wallet: Wallet,
    /// Amount of tokens to create  
    pub amount: f64,
    /// Optional metadata (defaults to empty object in JS - meta = null)
    pub meta: Option<HashMap<String, Value>>,
}

/// Mutation for creating new tokens
/// 
/// This mutation extends ProposeMolecule to handle token creation operations.
/// It initializes a molecule with token creation atoms and handles the 
/// submission process.
pub struct MutationCreateToken {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationCreateToken {
    /// Create a new MutationCreateToken (matches JS constructor)
    /// Takes graphQLClient, knishIOClient, and molecule like JS constructor
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationCreateToken {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility and simpler usage)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationCreateToken {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with token creation data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ recipientWallet, amount, meta = null })
    pub fn fill_molecule(&mut self, params: CreateTokenParams) -> crate::error::Result<()> {
        // Call molecule's initTokenCreation method (matches JS: this.$__molecule.initTokenCreation({recipientWallet, amount, meta: meta || {}}))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            // Convert meta HashMap to Vec<MetaItem>
            let meta_items = params.meta.unwrap_or_default().into_iter()
                .map(|(k, v)| MetaItem::new(&k, &v.to_string()))
                .collect();
            
            molecule.init_token_creation(&params.recipient_wallet, params.amount, meta_items)?;
            
            // Sign with recipient wallet bundle (matches JS: this.$__molecule.sign({bundle: recipientWallet.bundle}))
            let bundle = params.recipient_wallet.bundle.as_deref().unwrap_or("").to_string();
            molecule.sign(Some(bundle), false, true)?;
            
            // Check molecule (matches JS: this.$__molecule.check())
            molecule.check(None)?;
        }
        
        Ok(())
    }
    
    /// Create from token creation parameters
    pub fn from_params(params: CreateTokenParams, secret: &str) -> crate::error::Result<Self> {
        let mut molecule = Molecule::with_params(
            Some(secret.to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        
        // Convert meta HashMap to Vec<MetaItem>
        let meta_items = params.meta.clone().unwrap_or_default().into_iter()
            .map(|(k, v)| MetaItem::new(&k, &v.to_string()))
            .collect();
        
        // Initialize token creation in molecule
        molecule.init_token_creation(&params.recipient_wallet, params.amount, meta_items)?;
        let bundle = params.recipient_wallet.bundle.as_deref().unwrap_or("").to_string();
        molecule.sign(Some(bundle), false, true)?;
        molecule.check(None)?;
        
        Ok(Self::from_molecule(molecule))
    }
}

#[async_trait::async_trait]
impl Query for MutationCreateToken {
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
        // Using ResponseCreateToken for proper type safety
        Box::new(ResponseCreateToken::new(json))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationCreateToken {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationCreateToken {
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
    use serde_json::json;
    
    #[test]
    fn test_mutation_create_token_creation() {
        let molecule = Molecule::new();
        let mutation = MutationCreateToken::from_molecule(molecule);
        
        assert!(mutation.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_mutation_string() {
        let molecule = Molecule::new();
        let mutation = MutationCreateToken::from_molecule(molecule);
        let mutation_string = mutation.get_mutation();
        
        // Should have the same mutation string as ProposeMolecule
        assert!(mutation_string.contains("mutation( $molecule: MoleculeInput! )"));
        assert!(mutation_string.contains("ProposeMolecule( molecule: $molecule )"));
    }
    
    #[test]
    fn test_create_token_params() {
        let recipient_wallet = Wallet::create(
            Some("recipient-secret"),
            None,
            "TEST",
            None,
            None,
        ).unwrap();
        
        let mut meta = HashMap::new();
        meta.insert("name".to_string(), json!("TestToken"));
        meta.insert("symbol".to_string(), json!("TST"));
        
        let params = CreateTokenParams {
            recipient_wallet,
            amount: 1000.0,
            meta: Some(meta),
        };
        
        assert_eq!(params.amount, 1000.0);
        assert!(params.meta.is_some());
        assert_eq!(params.meta.as_ref().unwrap().len(), 2);
    }
    
    #[test]
    fn test_token_creation_flow() {
        let recipient_wallet = Wallet::create(
            Some("recipient-secret"),
            None,
            "TEST",
            None,
            None,
        ).unwrap();
        
        let params = CreateTokenParams {
            recipient_wallet,
            amount: 500.0,
            meta: None,
        };
        
        // This should fail without proper molecule initialization methods implemented
        let result = MutationCreateToken::from_params(params, "test-secret");
        
        // Expected to fail until molecule methods are implemented
        assert!(result.is_err());
    }
}