//! QueryWalletBundle implementation
//!
//! Query for retrieving information about Wallet Bundles,
//! equivalent to QueryWalletBundle.js

use crate::query::Query;
use crate::response::{Response, ResponseWalletBundle};
use serde_json::{json, Value};

/// Query for retrieving information about Wallet Bundles
pub struct QueryWalletBundle {
    /// Array of bundle hashes to query
    bundle_hashes: Vec<String>,
}

impl QueryWalletBundle {
    /// Create a new QueryWalletBundle instance
    pub fn new() -> Self {
        QueryWalletBundle {
            bundle_hashes: Vec::new(),
        }
    }

    /// Create a new QueryWalletBundle with bundle hashes
    pub fn with_bundle_hashes(bundle_hashes: Vec<String>) -> Self {
        QueryWalletBundle { bundle_hashes }
    }

    /// Add a bundle hash to the query
    pub fn add_bundle_hash(mut self, bundle_hash: impl Into<String>) -> Self {
        self.bundle_hashes.push(bundle_hash.into());
        self
    }

    /// Add multiple bundle hashes to the query
    pub fn add_bundle_hashes(mut self, bundle_hashes: Vec<String>) -> Self {
        self.bundle_hashes.extend(bundle_hashes);
        self
    }

    /// Set the bundle hashes (replacing any existing ones)
    pub fn set_bundle_hashes(&mut self, bundle_hashes: Vec<String>) {
        self.bundle_hashes = bundle_hashes;
    }

    /// Get the bundle hashes
    pub fn bundle_hashes(&self) -> &[String] {
        &self.bundle_hashes
    }

    /// Clear all bundle hashes
    pub fn clear_bundle_hashes(&mut self) {
        self.bundle_hashes.clear();
    }
}

impl Default for QueryWalletBundle {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for QueryWalletBundle {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query( $bundleHashes: [ String! ] ) {
          WalletBundle( bundleHashes: $bundleHashes ) {
            bundleHash,
            metas {
              molecularHash,
              position,
              key,
              value,
              createdAt
            },
            createdAt
          }
        }"#
    }

    /// Compile variables for the query (equivalent to compiledVariables in JS)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        if let Some(provided_vars) = variables {
            Some(provided_vars)
        } else {
            // Use instance bundle_hashes parameter
            Some(json!({
                "bundleHashes": self.bundle_hashes
            }))
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        match ResponseWalletBundle::new(json, None) {
            Ok(resp) => Box::new(resp),
            Err(e) => {
                eprintln!("ResponseWalletBundle construction failed: {}", e);
                Box::new(crate::response::BaseResponse::empty())
            }
        }
    }
}

/// Convenience methods for common usage patterns
impl QueryWalletBundle {
    /// Create a query for a single bundle hash
    pub fn by_bundle(bundle_hash: impl Into<String>) -> Self {
        Self::new().add_bundle_hash(bundle_hash)
    }

    /// Create a query for multiple bundle hashes
    pub fn by_bundles(bundle_hashes: Vec<String>) -> Self {
        Self::with_bundle_hashes(bundle_hashes)
    }

    /// Create a query from a slice of bundle hashes
    pub fn from_slice(bundle_hashes: &[impl AsRef<str>]) -> Self {
        let hashes: Vec<String> = bundle_hashes
            .iter()
            .map(|h| h.as_ref().to_string())
            .collect();
        Self::with_bundle_hashes(hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_wallet_bundle_creation() {
        let query = QueryWalletBundle::new();
        assert_eq!(query.bundle_hashes().len(), 0);
    }

    #[test]
    fn test_query_wallet_bundle_with_bundle_hashes() {
        let hashes = vec!["hash1".to_string(), "hash2".to_string()];
        let query = QueryWalletBundle::with_bundle_hashes(hashes.clone());
        assert_eq!(query.bundle_hashes(), hashes.as_slice());
    }

    #[test]
    fn test_add_bundle_hash() {
        let query = QueryWalletBundle::new()
            .add_bundle_hash("hash1")
            .add_bundle_hash("hash2");

        assert_eq!(query.bundle_hashes().len(), 2);
        assert_eq!(query.bundle_hashes()[0], "hash1");
        assert_eq!(query.bundle_hashes()[1], "hash2");
    }

    #[test]
    fn test_add_bundle_hashes() {
        let initial_hashes = vec!["hash1".to_string()];
        let additional_hashes = vec!["hash2".to_string(), "hash3".to_string()];
        
        let query = QueryWalletBundle::with_bundle_hashes(initial_hashes)
            .add_bundle_hashes(additional_hashes);

        assert_eq!(query.bundle_hashes().len(), 3);
        assert_eq!(query.bundle_hashes()[0], "hash1");
        assert_eq!(query.bundle_hashes()[1], "hash2");
        assert_eq!(query.bundle_hashes()[2], "hash3");
    }

    #[test]
    fn test_set_bundle_hashes() {
        let mut query = QueryWalletBundle::new().add_bundle_hash("original");
        let new_hashes = vec!["new1".to_string(), "new2".to_string()];
        
        query.set_bundle_hashes(new_hashes.clone());
        assert_eq!(query.bundle_hashes(), new_hashes.as_slice());
    }

    #[test]
    fn test_clear_bundle_hashes() {
        let mut query = QueryWalletBundle::new()
            .add_bundle_hash("hash1")
            .add_bundle_hash("hash2");
        
        assert_eq!(query.bundle_hashes().len(), 2);
        query.clear_bundle_hashes();
        assert_eq!(query.bundle_hashes().len(), 0);
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_bundle
        let query = QueryWalletBundle::by_bundle("single-hash");
        assert_eq!(query.bundle_hashes().len(), 1);
        assert_eq!(query.bundle_hashes()[0], "single-hash");

        // Test by_bundles
        let hashes = vec!["hash1".to_string(), "hash2".to_string()];
        let query = QueryWalletBundle::by_bundles(hashes.clone());
        assert_eq!(query.bundle_hashes(), hashes.as_slice());

        // Test from_slice
        let slice = ["hash1", "hash2"];
        let query = QueryWalletBundle::from_slice(&slice);
        assert_eq!(query.bundle_hashes().len(), 2);
        assert_eq!(query.bundle_hashes()[0], "hash1");
        assert_eq!(query.bundle_hashes()[1], "hash2");
    }

    #[test]
    fn test_compiled_variables() {
        let hashes = vec!["hash1".to_string(), "hash2".to_string()];
        let query = QueryWalletBundle::with_bundle_hashes(hashes.clone());
        let variables = query.compiled_variables(None).unwrap();
        
        assert_eq!(variables["bundleHashes"], json!(hashes));
    }

    #[test]
    fn test_compiled_variables_with_provided() {
        let query = QueryWalletBundle::new();
        let provided_vars = json!({
            "bundleHashes": ["provided-hash1", "provided-hash2"]
        });
        let variables = query.compiled_variables(Some(provided_vars)).unwrap();
        
        assert_eq!(variables["bundleHashes"], json!(["provided-hash1", "provided-hash2"]));
    }

    #[test]
    fn test_query_string() {
        let query = QueryWalletBundle::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("WalletBundle( bundleHashes: $bundleHashes )"));
        assert!(query_string.contains("bundleHash"));
        assert!(query_string.contains("metas"));
        assert!(query_string.contains("molecularHash"));
        assert!(query_string.contains("position"));
        assert!(query_string.contains("key"));
        assert!(query_string.contains("value"));
        assert!(query_string.contains("createdAt"));
    }
}