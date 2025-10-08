//! MutationRequestTokens implementation
//!
//! Mutation for requesting tokens from the network,
//! equivalent to MutationRequestTokens.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseRequestTokens};
use crate::molecule::Molecule;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use crate::types::MetaItem;
use serde_json::Value;
use std::collections::HashMap;

/// Parameters for fillMolecule (matches JS MutationRequestTokens fillMolecule parameters)
/// JS: fillMolecule({ token, amount, metaType, metaId, meta = null, batchId = null })
#[derive(Debug, Clone)]
pub struct RequestTokensParams {
    /// The token to request
    pub token: String,
    /// The requested amount
    pub amount: f64,
    /// The meta type
    pub meta_type: String,
    /// The meta ID
    pub meta_id: String,
    /// Optional metadata (defaults to empty object in JS - meta = null)
    pub meta: Option<HashMap<String, Value>>,
    /// Optional batch ID (defaults to null in JS - batchId = null)
    pub batch_id: Option<String>,
}

/// Mutation for requesting tokens
pub struct MutationRequestTokens {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationRequestTokens {
    /// Create a new MutationRequestTokens (matches JS constructor)
    /// Takes graphQLClient, knishIOClient, and molecule like JS constructor
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationRequestTokens {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility and simpler usage)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationRequestTokens {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with token request data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ token, amount, metaType, metaId, meta = null, batchId = null })
    pub fn fill_molecule(&mut self, params: RequestTokensParams) -> crate::error::Result<()> {
        // Convert HashMap to Vec<MetaItem>
        let meta_items: Vec<MetaItem> = if let Some(meta_map) = params.meta {
            meta_map.iter()
                .map(|(k, v)| MetaItem::new(k, &v.to_string()))
                .collect()
        } else {
            Vec::new()
        };
        
        // Call molecule's initTokenRequest method (matches JS: this.$__molecule.initTokenRequest({token, amount, metaType, metaId, meta: meta || {}, batchId}))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            molecule.init_token_request(
                &params.token,
                params.amount,
                &params.meta_type,
                &params.meta_id,
                meta_items,
                params.batch_id
            )?;
            
            // Sign with empty params (matches JS: this.$__molecule.sign({}))
            molecule.sign(
                None, // empty bundle for sign({})
                false, // anonymous = false
                true   // compressed = true
            )?;
            
            // Check molecule (matches JS: this.$__molecule.check())
            molecule.check(None)?;
        }
        
        Ok(())
    }
    
}

#[async_trait::async_trait]
impl Query for MutationRequestTokens {
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
        Box::new(ResponseRequestTokens::new(json))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationRequestTokens {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_request_tokens_creation() {
        let molecule = Molecule::new();
        let mutation = MutationRequestTokens::from_molecule(molecule);
        
        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_request_params() {
        let params = RequestTokensParams {
            token: "KNISH".to_string(),
            amount: 1000.0,
            meta_type: "TestMeta".to_string(),
            meta_id: "test123".to_string(),
            meta: None,
            batch_id: None,
        };
        
        assert_eq!(params.token, "KNISH");
        assert_eq!(params.amount, 1000.0);
        assert_eq!(params.meta_type, "TestMeta");
        assert_eq!(params.meta_id, "test123");
    }
}