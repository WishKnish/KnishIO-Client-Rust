//! QueryBalance implementation
//!
//! Query for getting the balance of a given wallet or token slug,
//! equivalent to QueryBalance.js

use crate::query::Query;
use crate::response::{Response, ResponseBalance};
use serde_json::{json, Value};

/// Query for getting the balance of a given wallet or token slug
pub struct QueryBalance {
    /// Optional wallet address to query
    address: Option<String>,
    /// Optional bundle hash to query
    bundle_hash: Option<String>,
    /// Optional wallet type to filter by
    wallet_type: Option<String>,
    /// Optional token slug to filter by
    token: Option<String>,
    /// Optional position to filter by
    position: Option<String>,
}

impl QueryBalance {
    /// Create a new QueryBalance instance
    pub fn new() -> Self {
        QueryBalance {
            address: None,
            bundle_hash: None,
            wallet_type: None,
            token: None,
            position: None,
        }
    }

    /// Set the wallet address parameter
    pub fn with_address(mut self, address: impl Into<String>) -> Self {
        self.address = Some(address.into());
        self
    }

    /// Set the bundle hash parameter
    pub fn with_bundle_hash(mut self, bundle_hash: impl Into<String>) -> Self {
        self.bundle_hash = Some(bundle_hash.into());
        self
    }

    /// Set the wallet type parameter
    pub fn with_type(mut self, wallet_type: impl Into<String>) -> Self {
        self.wallet_type = Some(wallet_type.into());
        self
    }

    /// Set the token slug parameter
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the position parameter
    pub fn with_position(mut self, position: impl Into<String>) -> Self {
        self.position = Some(position.into());
        self
    }
}

impl Default for QueryBalance {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for QueryBalance {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query( $address: String, $bundleHash: String, $type: String, $token: String, $position: String ) {
          Balance( address: $address, bundleHash: $bundleHash, type: $type, token: $token, position: $position ) {
            address,
            bundleHash,
            type,
            tokenSlug,
            batchId,
            position,
            amount,
            characters,
            pubkey,
            createdAt,
            tokenUnits {
              id,
              name,
              metas
            },
            tradeRates {
              tokenSlug,
              amount
            }
          }
        }"#
    }

    /// Compile variables for the query (equivalent to compiledVariables in JS)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        let mut vars = json!({});

        // Use provided variables if available, otherwise use instance variables
        if let Some(provided_vars) = variables {
            vars = provided_vars;
        } else {
            // Build variables from instance properties
            if let Some(ref address) = self.address {
                vars["address"] = json!(address);
            }
            if let Some(ref bundle_hash) = self.bundle_hash {
                vars["bundleHash"] = json!(bundle_hash);
            }
            if let Some(ref wallet_type) = self.wallet_type {
                vars["type"] = json!(wallet_type);
            }
            if let Some(ref token) = self.token {
                vars["token"] = json!(token);
            }
            if let Some(ref position) = self.position {
                vars["position"] = json!(position);
            }
        }

        Some(vars)
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseBalance::new(json, None).expect("Failed to create ResponseBalance"))
    }
}

/// Convenience functions for common query patterns
impl QueryBalance {
    /// Query balance by token slug (most common pattern)
    pub fn by_token(token: impl Into<String>) -> Self {
        Self::new().with_token(token)
    }

    /// Query balance by wallet address
    pub fn by_address(address: impl Into<String>) -> Self {
        Self::new().with_address(address)
    }

    /// Query balance by bundle hash
    pub fn by_bundle(bundle_hash: impl Into<String>) -> Self {
        Self::new().with_bundle_hash(bundle_hash)
    }

    /// Query balance by token and bundle (common pattern)
    pub fn by_token_and_bundle(token: impl Into<String>, bundle_hash: impl Into<String>) -> Self {
        Self::new()
            .with_token(token)
            .with_bundle_hash(bundle_hash)
    }

    /// Query balance by token and address (common pattern)
    pub fn by_token_and_address(token: impl Into<String>, address: impl Into<String>) -> Self {
        Self::new()
            .with_token(token)
            .with_address(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_balance_creation() {
        let query = QueryBalance::new();
        assert!(query.address.is_none());
        assert!(query.token.is_none());
    }

    #[test]
    fn test_query_balance_with_parameters() {
        let query = QueryBalance::new()
            .with_token("KNISH")
            .with_address("test-address");

        assert_eq!(query.token, Some("KNISH".to_string()));
        assert_eq!(query.address, Some("test-address".to_string()));
    }

    #[test]
    fn test_query_balance_convenience_methods() {
        let query = QueryBalance::by_token("KNISH");
        assert_eq!(query.token, Some("KNISH".to_string()));

        let query = QueryBalance::by_address("test-address");
        assert_eq!(query.address, Some("test-address".to_string()));

        let query = QueryBalance::by_token_and_bundle("KNISH", "bundle-hash");
        assert_eq!(query.token, Some("KNISH".to_string()));
        assert_eq!(query.bundle_hash, Some("bundle-hash".to_string()));
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryBalance::new()
            .with_token("KNISH")
            .with_address("test-address");

        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["token"], json!("KNISH"));
        assert_eq!(variables["address"], json!("test-address"));
    }

    #[test]
    fn test_query_string() {
        let query = QueryBalance::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("Balance("));
        assert!(query_string.contains("address"));
        assert!(query_string.contains("bundleHash"));
        assert!(query_string.contains("tokenSlug"));
        assert!(query_string.contains("tokenUnits"));
        assert!(query_string.contains("tradeRates"));
    }
}