//! MutationActiveSession implementation
//!
//! Mutation for declaring an active User Session with a given MetaAsset,
//! equivalent to MutationActiveSession.js

use crate::mutation::Mutation;
use crate::query::Query;
use crate::response::{Response, ResponseActiveSession};
use serde_json::Value;

/// Mutation for declaring an active User Session
pub struct MutationActiveSession {
}

impl MutationActiveSession {
    /// Create a new MutationActiveSession instance
    pub fn new() -> Self {
        MutationActiveSession {}
    }
}

impl Default for MutationActiveSession {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for MutationActiveSession {
    /// Get the GraphQL mutation string
    fn get_query(&self) -> &str {
        r#"mutation(
          $bundleHash: String!,
          $metaType: String!,
          $metaId: String!,
          $ipAddress: String,
          $browser: String,
          $osCpu: String,
          $resolution: String,
          $timeZone: String,
          $json: String
        ) {
          ActiveSession(
            bundleHash: $bundleHash,
            metaType: $metaType,
            metaId: $metaId,
            ipAddress: $ipAddress,
            browser: $browser,
            osCpu: $osCpu,
            resolution: $resolution,
            timeZone: $timeZone,
            json: $json
          ) {
            bundleHash,
            metaType,
            metaId,
            jsonData,
            createdAt,
            updatedAt
          }
        }"#
    }
    
    /// Compile variables for the mutation (pass through)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        variables
    }
    
    /// Create a response from the JSON data
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseActiveSession::new(json, None).expect("Failed to create ResponseActiveSession"))
    }
}

#[async_trait::async_trait]
impl Mutation for MutationActiveSession {
    /// Get the GraphQL mutation string
    fn get_mutation(&self) -> &str {
        self.get_query()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutation_active_session_creation() {
        let mutation = MutationActiveSession::new();
        let mutation_string = mutation.get_mutation();
        
        // Check that the mutation string contains expected fields
        assert!(mutation_string.contains("mutation("));
        assert!(mutation_string.contains("$bundleHash: String!"));
        assert!(mutation_string.contains("$metaType: String!"));
        assert!(mutation_string.contains("$metaId: String!"));
        assert!(mutation_string.contains("ActiveSession("));
        assert!(mutation_string.contains("bundleHash"));
        assert!(mutation_string.contains("metaType"));
        assert!(mutation_string.contains("metaId"));
        assert!(mutation_string.contains("jsonData"));
        assert!(mutation_string.contains("createdAt"));
        assert!(mutation_string.contains("updatedAt"));
    }
}