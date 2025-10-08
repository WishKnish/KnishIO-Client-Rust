//! MutationLinkIdentifier implementation
//!
//! Mutation for linking an Identifier to a Wallet Bundle,
//! equivalent to MutationLinkIdentifier.js

use crate::mutation::Mutation;
use crate::query::Query;
use crate::response::{Response, ResponseLinkIdentifier};
use serde_json::Value;

/// Mutation for linking an Identifier to a Wallet Bundle
pub struct MutationLinkIdentifier {
}

impl MutationLinkIdentifier {
    /// Create a new MutationLinkIdentifier instance
    pub fn new() -> Self {
        MutationLinkIdentifier {}
    }
}

#[async_trait::async_trait]
impl Query for MutationLinkIdentifier {
    /// Get the GraphQL mutation string
    fn get_query(&self) -> &str {
        r#"mutation( $bundle: String!, $type: String!, $content: String! ) {
          LinkIdentifier( bundle: $bundle, type: $type, content: $content ) {
            type,
            bundle,
            content,
            set,
            message
          }
        }"#
    }
    
    /// Compile variables for the mutation (pass through)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        variables
    }
    
    /// Create a response from the JSON data
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseLinkIdentifier::new(json, None).expect("Failed to create ResponseLinkIdentifier"))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationLinkIdentifier {
    /// Get the GraphQL mutation string
    fn get_mutation(&self) -> &str {
        self.get_query()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_link_identifier_creation() {
        let mutation = MutationLinkIdentifier::new();
        let mutation_string = mutation.get_mutation();
        
        // Check that the mutation string contains expected fields
        assert!(mutation_string.contains("mutation( $bundle: String!, $type: String!, $content: String! )"));
        assert!(mutation_string.contains("LinkIdentifier( bundle: $bundle, type: $type, content: $content )"));
        assert!(mutation_string.contains("type"));
        assert!(mutation_string.contains("bundle"));
        assert!(mutation_string.contains("content"));
        assert!(mutation_string.contains("set"));
        assert!(mutation_string.contains("message"));
    }
}