//! MutationCreateIdentifier implementation
//!
//! Mutation for creating a new Identifier,
//! equivalent to MutationCreateIdentifier.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseCreateIdentifier};
use crate::molecule::Molecule;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::Value;

/// Parameters for fillMolecule (matches JS MutationCreateIdentifier fillMolecule parameters)
/// JS: fillMolecule({ type, contact, code })
#[derive(Debug, Clone)]
pub struct CreateIdentifierParams {
    /// The identifier type (matches JS 'type' parameter)
    pub r#type: String,
    /// The contact information
    pub contact: String,
    /// The verification code
    pub code: String,
}

/// Mutation for creating a new Identifier
pub struct MutationCreateIdentifier {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationCreateIdentifier {
    /// Create a new MutationCreateIdentifier (matches JS constructor)
    /// Takes graphQLClient, knishIOClient, and molecule like JS constructor
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationCreateIdentifier {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility and simpler usage)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationCreateIdentifier {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with identifier creation data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ type, contact, code })
    pub fn fill_molecule(&mut self, params: CreateIdentifierParams) -> crate::error::Result<()> {
        // Call molecule's initIdentifierCreation method (matches JS: this.$__molecule.initIdentifierCreation({type, contact, code}))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            molecule.init_identifier_creation(
                &params.r#type,
                &params.contact,
                &params.code
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
impl Query for MutationCreateIdentifier {
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
        Box::new(ResponseCreateIdentifier::new(json, None).expect("Failed to create ResponseCreateIdentifier"))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationCreateIdentifier {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationCreateIdentifier {
    /// Get the underlying molecule
    pub fn molecule(&self) -> &Molecule {
        self.propose_molecule.molecule()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_create_identifier_creation() {
        let molecule = Molecule::new();
        let mutation = MutationCreateIdentifier::from_molecule(molecule);
        
        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_create_identifier_params() {
        let params = CreateIdentifierParams {
            r#type: "email".to_string(),
            contact: "user@example.com".to_string(),
            code: "123456".to_string(),
        };
        
        assert_eq!(params.r#type, "email");
        assert_eq!(params.contact, "user@example.com");
        assert_eq!(params.code, "123456");
    }
}