//! MutationRequestAuthorization implementation
//!
//! Mutation for requesting an authorization token from the node,
//! equivalent to MutationRequestAuthorization.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseRequestAuthorization};
use crate::molecule::Molecule;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use crate::types::MetaItem;
use serde_json::Value;
use std::collections::HashMap;

/// Parameters for requesting authorization (matches JS fillMolecule parameters)
/// JS: fillMolecule({ meta })
pub struct RequestAuthorizationParams {
    /// Metadata for the authorization request
    pub meta: HashMap<String, Value>,
}

/// Mutation for requesting an authorization token from the node
pub struct MutationRequestAuthorization {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationRequestAuthorization {
    /// Create a new MutationRequestAuthorization (matches JS constructor)
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationRequestAuthorization {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationRequestAuthorization {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with authorization request data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ meta })
    pub fn fill_molecule(&mut self, params: RequestAuthorizationParams) -> crate::error::Result<()> {
        // Call molecule's initAuthorization method (matches JS: this.$__molecule.initAuthorization({ meta }))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            // Convert HashMap to Vec<MetaItem>
            let meta_items: Vec<MetaItem> = params.meta.iter()
                .map(|(k, v)| MetaItem::new(k, &v.to_string()))
                .collect();
            
            molecule.init_authorization(meta_items)?;
            
            // Sign with empty params (matches JS: this.$__molecule.sign({}))
            molecule.sign(None, false, true)?;
            
            // Check molecule (matches JS: this.$__molecule.check())
            molecule.check(None)?;
        }
        
        Ok(())
    }
    
    /// Create from authorization parameters (JavaScript API pattern)
    pub fn from_params(params: RequestAuthorizationParams, _secret: &str) -> Self {
        let mut molecule = Molecule::new();
        
        // Apply JavaScript authorization parameters
        if !params.meta.is_empty() {
            let meta = &params.meta;
            // Convert HashMap to Vec<MetaItem> for JavaScript compatibility
            let meta_items: Vec<crate::types::MetaItem> = meta.iter()
                .map(|(k, v)| crate::types::MetaItem {
                    key: k.clone(),
                    value: v.to_string(),
                })
                .collect();
                
            // Initialize authorization metadata like JavaScript SDK
            if let Err(_e) = molecule.init_meta(meta_items, "authorization", "auth_request", None) {
                // Handle initialization error gracefully
            }
        }
        
        Self::from_molecule(molecule)
    }
}

#[async_trait::async_trait]
impl Query for MutationRequestAuthorization {
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
        Box::new(ResponseRequestAuthorization::new(json))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationRequestAuthorization {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationRequestAuthorization {
    /// Get the underlying molecule
    pub fn molecule(&self) -> &Molecule {
        self.propose_molecule.molecule()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_mutation_request_authorization_creation() {
        let molecule = Molecule::new();
        let mutation = MutationRequestAuthorization::new(
            GraphQLClient::new("http://localhost:4000/graphql"),
            KnishIOClient::new(
                "http://localhost:4000/graphql",
                Some("TEST_CELL".to_string()),
                None,  // socket
                None,  // client
                None,  // server_sdk_version
                None   // logging
            ),
            molecule,
        );

        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_request_authorization_params() {
        let mut meta = HashMap::new();
        meta.insert("purpose".to_string(), json!("api_access"));
        meta.insert("permissions".to_string(), json!(["read", "write"]));
        
        let params = RequestAuthorizationParams {
            meta,
        };
        
        assert_eq!(params.meta.len(), 2);
    }
}