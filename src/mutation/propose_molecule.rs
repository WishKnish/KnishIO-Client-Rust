//! MutationProposeMolecule implementation
//!
//! Mutation for proposing a molecule to the network,
//! equivalent to MutationProposeMolecule.js

use crate::mutation::Mutation;
use crate::query::Query;
use crate::response::{Response, ResponseProposeMolecule};
use crate::molecule::Molecule;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Mutation for proposing a molecule to the network
/// 
/// This is the most critical mutation as it handles all transaction submissions
/// to the KnishIO distributed ledger. All other mutation types ultimately use
/// this to submit their molecular transactions.
pub struct MutationProposeMolecule {
    /// The molecule to propose
    molecule: Molecule,
    /// Optional remainder wallet
    remainder_wallet: Option<Value>,
    /// GraphQL client reference
    #[allow(dead_code)]
    graph_ql_client: Option<GraphQLClient>,
    /// KnishIO client reference  
    #[allow(dead_code)]
    knish_io_client: Option<KnishIOClient>,
}

impl MutationProposeMolecule {
    /// Create a new MutationProposeMolecule instance (matches JS constructor)
    /// Takes graphQLClient, knishIOClient, molecule like JS constructor:
    /// constructor (graphQLClient, knishIOClient, molecule)
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationProposeMolecule {
            molecule,
            remainder_wallet: None,
            graph_ql_client: Some(graph_ql_client),
            knish_io_client: Some(knish_io_client),
        }
    }
    
    /// Create with just molecule (for backward compatibility)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationProposeMolecule {
            molecule,
            remainder_wallet: None,
            graph_ql_client: None,
            knish_io_client: None,
        }
    }
    
    /// Set the remainder wallet
    pub fn with_remainder_wallet(mut self, wallet: Value) -> Self {
        self.remainder_wallet = Some(wallet);
        self
    }
    
    /// Get the molecule (matches JS molecule())
    pub fn molecule(&self) -> &Molecule {
        &self.molecule
    }
    
    /// Get mutable access to molecule
    pub fn get_molecule_mut(&mut self) -> Option<&mut Molecule> {
        Some(&mut self.molecule)
    }
    
    /// Get the remainder wallet
    pub fn remainder_wallet(&self) -> Option<&Value> {
        self.remainder_wallet.as_ref()
    }
}

#[async_trait::async_trait]
impl Query for MutationProposeMolecule {
    /// Get the GraphQL query string (mutation in this case)
    fn get_query(&self) -> &str {
        r#"mutation( $molecule: MoleculeInput! ) {
          ProposeMolecule( molecule: $molecule ) {
            molecularHash,
            height,
            depth,
            status,
            reason,
            payload,
            createdAt,
            receivedAt,
            processedAt,
            broadcastedAt,
          }
        }"#
    }
    
    /// Compile variables for the mutation
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        let mut vars = variables.unwrap_or_else(|| json!({}));
        
        // Add the molecule to variables
        vars["molecule"] = self.molecule.to_json(crate::types::MoleculeJsonOptions::default()).unwrap_or_default();
        
        Some(vars)
    }
    
    /// Create a response from the JSON data
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        // Using ResponseProposeMolecule for proper type safety
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
impl Mutation for MutationProposeMolecule {
    /// Get the GraphQL mutation string
    fn get_mutation(&self) -> &str {
        self.get_query()
    }
    
    /// Execute the mutation with molecule included in variables
    async fn execute(
        &self,
        client: &crate::graphql::GraphQLClient,
        variables: Option<Value>,
        _context: Option<HashMap<String, Value>>,
    ) -> crate::error::Result<Box<dyn Response>> {
        // Ensure molecule is in variables
        let mut vars = variables.unwrap_or_else(|| json!({}));
        vars["molecule"] = self.molecule.to_json(crate::types::MoleculeJsonOptions::default()).unwrap_or_default();
        
        // Use the default Mutation execute
        let request = self.create_mutation_request(Some(vars));
        
        let response = client.mutate(request).await?;
        
        // Convert GraphQLResponse to our Response type
        let json_data = response.data.unwrap_or_else(|| json!({}));
        Ok(self.create_response(json_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_propose_molecule_creation() {
        // Create a mock molecule
        let molecule = Molecule::new();
        let mutation = MutationProposeMolecule::from_molecule(molecule);
        
        assert!(mutation.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_mutation_string() {
        let molecule = Molecule::new();
        let mutation = MutationProposeMolecule::from_molecule(molecule);
        let mutation_string = mutation.get_mutation();
        
        // Check that the mutation string contains expected fields
        assert!(mutation_string.contains("mutation( $molecule: MoleculeInput! )"));
        assert!(mutation_string.contains("ProposeMolecule( molecule: $molecule )"));
        assert!(mutation_string.contains("molecularHash"));
        assert!(mutation_string.contains("status"));
        assert!(mutation_string.contains("reason"));
        assert!(mutation_string.contains("createdAt"));
    }
    
    #[test]
    fn test_compiled_variables() {
        let molecule = Molecule::new();
        let mutation = MutationProposeMolecule::from_molecule(molecule);
        
        let variables = mutation.compiled_variables(None).unwrap();
        assert!(variables.get("molecule").is_some());
    }
    
    #[test]
    fn test_mutation_builder_pattern() {
        let molecule = Molecule::new();
        let mutation = MutationProposeMolecule::from_molecule(molecule)
            .with_remainder_wallet(json!({"test": "wallet"}));
        
        assert!(mutation.remainder_wallet().is_some());
    }
}