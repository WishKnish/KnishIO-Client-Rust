//! MutationCreateRule implementation
//!
//! Mutation for creating new Meta attached to some MetaType,
//! equivalent to MutationCreateRule.js

use crate::mutation::{Mutation, propose_molecule::MutationProposeMolecule};
use crate::query::Query;
use crate::response::{Response, ResponseCreateRule};
use crate::molecule::Molecule;
use crate::graphql::GraphQLClient;
use crate::client::KnishIOClient;
use serde_json::Value;

/// Parameters for fillMolecule (matches JS MutationCreateRule fillMolecule parameters)
/// JS: fillMolecule({ metaType, metaId, rule, policy })
#[derive(Debug, Clone)]
pub struct CreateRuleParams {
    /// The meta type
    pub meta_type: String,
    /// The meta ID
    pub meta_id: String,
    /// The rule definition
    pub rule: Vec<Value>,
    /// The policy definition
    pub policy: Value,
}

/// Mutation for creating new Meta attached to some MetaType
pub struct MutationCreateRule {
    /// The underlying propose molecule mutation
    propose_molecule: MutationProposeMolecule,
}

impl MutationCreateRule {
    /// Create a new MutationCreateRule (matches JS constructor)
    pub fn new(graph_ql_client: GraphQLClient, knish_io_client: KnishIOClient, molecule: Molecule) -> Self {
        MutationCreateRule {
            propose_molecule: MutationProposeMolecule::new(graph_ql_client, knish_io_client, molecule),
        }
    }
    
    /// Create with just molecule (for backward compatibility)
    pub fn from_molecule(molecule: Molecule) -> Self {
        MutationCreateRule {
            propose_molecule: MutationProposeMolecule::from_molecule(molecule),
        }
    }
    
    /// Fill the molecule with rule creation data (matches JS fillMolecule exactly)
    /// JS: fillMolecule({ metaType, metaId, rule, policy })
    pub fn fill_molecule(&mut self, params: CreateRuleParams) -> crate::error::Result<()> {
        // Call molecule's createRule method (matches JS: this.$__molecule.createRule({metaType, metaId, rule, policy}))
        if let Some(ref mut molecule) = self.propose_molecule.get_molecule_mut() {
            // Convert rule Vec<Value> to JSON string
            let rule_str = serde_json::to_string(&params.rule).unwrap_or_default();
            // Convert policy Value to Option<String>
            let policy_str = params.policy.to_string();
            
            molecule.create_rule(
                &params.meta_type,
                &params.meta_id,
                &rule_str,
                Some(&policy_str)
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
        // Note: This will be implemented once Molecule has the proper methods
        // self.propose_molecule.molecule.create_rule(
        //     &params.meta_type,
        //     &params.meta_id,
        //     &params.rule,
        //     &params.policy
        // );
        // self.propose_molecule.molecule.sign(None);
        // self.propose_molecule.molecule.check(None);
    }
    
    /// Create from rule parameters
    pub fn from_params(_params: CreateRuleParams, _secret: &str) -> Self {
        let molecule = Molecule::new();
        
        // Initialize rule creation in molecule
        // molecule.create_rule(
        //     &params.meta_type,
        //     &params.meta_id,
        //     &params.rule,
        //     &params.policy
        // );
        // molecule.sign(None);
        // molecule.check(None);
        
        Self::from_molecule(molecule)
    }
}

#[async_trait::async_trait]
impl Query for MutationCreateRule {
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
        Box::new(ResponseCreateRule::new(json))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationCreateRule {
    /// Delegate to the underlying propose molecule mutation
    fn get_mutation(&self) -> &str {
        self.propose_molecule.get_mutation()
    }
}

/// Convenience methods
impl MutationCreateRule {
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
    fn test_mutation_create_rule_creation() {
        let molecule = Molecule::new();
        let mutation = MutationCreateRule::from_molecule(molecule);
        
        // Test basic creation
        assert!(mutation.propose_molecule.remainder_wallet().is_none());
    }
    
    #[test]
    fn test_create_rule_params() {
        let params = CreateRuleParams {
            meta_type: "user".to_string(),
            meta_id: "user123".to_string(),
            rule: vec![json!({"condition": "email_verified"})],
            policy: json!({"enforce": true}),
        };
        
        assert_eq!(params.meta_type, "user");
        assert_eq!(params.meta_id, "user123");
        assert_eq!(params.rule.len(), 1);
    }
}