//! MutationCreateMeta implementation
//!
//! Mutation for creating metadata on the ledger,
//! equivalent to MutationCreateMeta.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseCreateMeta};
use crate::molecule::Molecule;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use crate::types::MetaItem;
use serde_json::Value;
use std::collections::HashMap;

/// Parameters for fillMolecule (matches JS MutationCreateMeta fillMolecule parameters)
/// JS: fillMolecule({ metaType, metaId, meta, policy })
#[derive(Debug, Clone)]
pub struct CreateMetaParams {
    /// The meta type
    pub meta_type: String,
    /// The meta ID
    pub meta_id: String,
    /// The metadata (array|object in JS)
    pub meta: HashMap<String, Value>,
    /// The policy object
    pub policy: HashMap<String, Value>,
}

/// Mutation for creating metadata
pub struct MutationCreateMeta {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationCreateMeta {
    /// Create a new MutationCreateMeta (matches JS constructor)
    /// Takes graphQLClient, knishIOClient, and molecule like JS constructor
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationCreateMeta {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility and simpler usage)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationCreateMeta {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with metadata (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ metaType, metaId, meta, policy })
    pub fn fill_molecule(&mut self, params: CreateMetaParams) -> crate::error::Result<()> {
        // Call molecule's initMeta method (matches JS: this.$__molecule.initMeta({meta, metaType, metaId, policy}))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            // Convert HashMap to Vec<MetaItem>
            let meta_items: Vec<MetaItem> = params.meta.iter()
                .map(|(k, v)| MetaItem::new(k, &v.to_string()))
                .collect();
            
            // Convert policy HashMap to JSON string or None
            let policy_str = if params.policy.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&params.policy).unwrap_or_default())
            };
            
            molecule.init_meta(
                meta_items,
                &params.meta_type,
                &params.meta_id,
                policy_str.as_deref()
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
impl Query for MutationCreateMeta {
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
        Box::new(ResponseCreateMeta::new(json))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationCreateMeta {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationCreateMeta {
    /// Get the underlying molecule
    pub fn molecule(&self) -> &Molecule {
        self.propose_molecule.molecule()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::*;
    
    #[test]
    fn test_mutation_create_meta_creation() {
        let molecule = Molecule::new();
        let mutation = MutationCreateMeta::from_molecule(molecule);
        
        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_create_meta_params() {
        let mut meta = HashMap::new();
        meta.insert("name".to_string(), json!("Test User"));
        meta.insert("email".to_string(), json!("test@example.com"));
        
        let mut policy = HashMap::new();
        policy.insert("access".to_string(), json!("public"));
        
        let params = CreateMetaParams {
            meta_type: "user".to_string(),
            meta_id: "user123".to_string(),
            meta,
            policy,
        };
        
        assert_eq!(params.meta_type, "user");
        assert_eq!(params.meta_id, "user123");
        assert_eq!(params.meta.len(), 2);
        assert_eq!(params.policy.len(), 1);
    }
}