//! QueryContinuId implementation
//!
//! Queries the node for the next wallet to sign with for ContinuID,
//! equivalent to QueryContinuId.js

use crate::query::Query;
use crate::response::{Response, ResponseContinuId};
use serde_json::{json, Value};

/// Queries the node for the next wallet to sign with for ContinuID
pub struct QueryContinuId {
    /// Bundle hash (required parameter)
    bundle: String,
}

impl QueryContinuId {
    /// Create a new QueryContinuId instance with bundle hash
    pub fn new(bundle: impl Into<String>) -> Self {
        QueryContinuId {
            bundle: bundle.into(),
        }
    }

    /// Get the bundle hash
    pub fn bundle(&self) -> &str {
        &self.bundle
    }

    /// Set a new bundle hash
    pub fn set_bundle(&mut self, bundle: impl Into<String>) {
        self.bundle = bundle.into();
    }
}

#[async_trait::async_trait]
impl Query for QueryContinuId {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query ($bundle: String!) {
          ContinuId(bundle: $bundle) {
            address,
            bundleHash,
            tokenSlug,
            position,
            batchId,
            characters,
            pubkey,
            amount,
            createdAt
          }
        }"#
    }

    /// Compile variables for the query (equivalent to compiledVariables in JS)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        if let Some(provided_vars) = variables {
            Some(provided_vars)
        } else {
            // Use instance bundle parameter
            Some(json!({
                "bundle": self.bundle
            }))
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseContinuId::new(json, None).expect("Failed to create ResponseContinuId"))
    }
}

/// Convenience methods for common usage patterns
impl QueryContinuId {
    /// Create a query by bundle hash (most common pattern)
    pub fn by_bundle(bundle: impl Into<String>) -> Self {
        Self::new(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_continu_id_creation() {
        let query = QueryContinuId::new("test-bundle-hash");
        assert_eq!(query.bundle(), "test-bundle-hash");
    }

    #[test]
    fn test_query_continu_id_by_bundle() {
        let query = QueryContinuId::by_bundle("test-bundle");
        assert_eq!(query.bundle(), "test-bundle");
    }

    #[test]
    fn test_set_bundle() {
        let mut query = QueryContinuId::new("original-bundle");
        query.set_bundle("new-bundle");
        assert_eq!(query.bundle(), "new-bundle");
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryContinuId::new("test-bundle-hash");
        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["bundle"], json!("test-bundle-hash"));
    }

    #[test]
    fn test_compiled_variables_with_provided() {
        let query = QueryContinuId::new("test-bundle-hash");
        let provided_vars = json!({
            "bundle": "provided-bundle"
        });
        let variables = query.compiled_variables(Some(provided_vars)).unwrap();
        assert_eq!(variables["bundle"], json!("provided-bundle"));
    }

    #[test]
    fn test_query_string() {
        let query = QueryContinuId::new("test-bundle");
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("ContinuId(bundle: $bundle)"));
        assert!(query_string.contains("address"));
        assert!(query_string.contains("bundleHash"));
        assert!(query_string.contains("tokenSlug"));
        assert!(query_string.contains("position"));
        assert!(query_string.contains("batchId"));
        assert!(query_string.contains("characters"));
        assert!(query_string.contains("pubkey"));
        assert!(query_string.contains("amount"));
        assert!(query_string.contains("createdAt"));
    }
}