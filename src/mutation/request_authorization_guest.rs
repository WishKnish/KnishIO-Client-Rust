//! MutationRequestAuthorizationGuest implementation
//!
//! Mutation for requesting guest authorization tokens,
//! equivalent to MutationRequestAuthorizationGuest.js

use crate::mutation::Mutation;
use crate::query::Query;
use crate::response::{Response, ResponseRequestAuthorizationGuest};
use serde_json::Value;

/// Mutation for requesting guest authorization tokens
pub struct MutationRequestAuthorizationGuest {
}

impl MutationRequestAuthorizationGuest {
    /// Create a new MutationRequestAuthorizationGuest instance
    pub fn new() -> Self {
        MutationRequestAuthorizationGuest {}
    }
}

impl Default for MutationRequestAuthorizationGuest {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for MutationRequestAuthorizationGuest {
    /// Get the GraphQL mutation string
    fn get_query(&self) -> &str {
        r#"mutation( $cellSlug: String, $pubkey: String, $encrypt: Boolean ) {
          AccessToken( cellSlug: $cellSlug, pubkey: $pubkey, encrypt: $encrypt ) {
            token,
            pubkey,
            expiresAt
          }
        }"#
    }
    
    /// Compile variables for the mutation (pass through)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        variables
    }
    
    /// Create a response from the JSON data
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseRequestAuthorizationGuest::new(json, None).expect("Failed to create ResponseRequestAuthorizationGuest"))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationRequestAuthorizationGuest {
    /// Get the GraphQL mutation string
    fn get_mutation(&self) -> &str {
        self.get_query()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_request_authorization_guest_creation() {
        let mutation = MutationRequestAuthorizationGuest::new();
        let mutation_string = mutation.get_mutation();
        
        // Check that the mutation string contains expected fields
        assert!(mutation_string.contains("mutation( $cellSlug: String, $pubkey: String, $encrypt: Boolean )"));
        assert!(mutation_string.contains("AccessToken( cellSlug: $cellSlug, pubkey: $pubkey, encrypt: $encrypt )"));
        assert!(mutation_string.contains("token"));
        assert!(mutation_string.contains("pubkey"));
        assert!(mutation_string.contains("expiresAt"));
    }
}