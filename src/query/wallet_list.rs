//! QueryWalletList implementation
//!
//! Query for getting a list of Wallets,
//! equivalent to QueryWalletList.js

use crate::query::Query;
use crate::response::{Response, ResponseWalletList};
use serde_json::{json, Value};

/// Query for getting a list of Wallets
pub struct QueryWalletList {
    /// Optional bundle hash to filter by
    bundle_hash: Option<String>,
    /// Optional token slug to filter by
    token_slug: Option<String>,
}

impl QueryWalletList {
    /// Create a new QueryWalletList instance
    pub fn new() -> Self {
        QueryWalletList {
            bundle_hash: None,
            token_slug: None,
        }
    }

    /// Set the bundle hash parameter
    pub fn with_bundle_hash(mut self, bundle_hash: impl Into<String>) -> Self {
        self.bundle_hash = Some(bundle_hash.into());
        self
    }

    /// Set the token slug parameter
    pub fn with_token_slug(mut self, token_slug: impl Into<String>) -> Self {
        self.token_slug = Some(token_slug.into());
        self
    }

    /// Set both bundle hash and token slug
    pub fn with_filters(mut self, bundle_hash: impl Into<String>, token_slug: impl Into<String>) -> Self {
        self.bundle_hash = Some(bundle_hash.into());
        self.token_slug = Some(token_slug.into());
        self
    }

    /// Get the bundle hash
    pub fn bundle_hash(&self) -> Option<&str> {
        self.bundle_hash.as_deref()
    }

    /// Get the token slug
    pub fn token_slug(&self) -> Option<&str> {
        self.token_slug.as_deref()
    }
}

impl Default for QueryWalletList {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for QueryWalletList {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query( $bundleHash: String, $tokenSlug: String ) {
          Wallet( bundleHash: $bundleHash, tokenSlug: $tokenSlug ) {
            address,
            bundleHash,
            token {
              name,
              amount,
              fungibility,
              supply
            },
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
        if let Some(provided_vars) = variables {
            Some(provided_vars)
        } else {
            let mut vars = json!({});

            if let Some(ref bundle_hash) = self.bundle_hash {
                vars["bundleHash"] = json!(bundle_hash);
            }
            if let Some(ref token_slug) = self.token_slug {
                vars["tokenSlug"] = json!(token_slug);
            }

            Some(vars)
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        match ResponseWalletList::new(json, None) {
            Ok(resp) => Box::new(resp),
            Err(e) => {
                eprintln!("ResponseWalletList construction failed: {}", e);
                Box::new(crate::response::BaseResponse::empty())
            }
        }
    }
}

/// Convenience methods for common usage patterns
impl QueryWalletList {
    /// Query wallets by bundle hash
    pub fn by_bundle_hash(bundle_hash: impl Into<String>) -> Self {
        Self::new().with_bundle_hash(bundle_hash)
    }

    /// Query wallets by token slug
    pub fn by_token_slug(token_slug: impl Into<String>) -> Self {
        Self::new().with_token_slug(token_slug)
    }

    /// Query wallets by both bundle hash and token slug
    pub fn by_bundle_and_token(bundle_hash: impl Into<String>, token_slug: impl Into<String>) -> Self {
        Self::new().with_filters(bundle_hash, token_slug)
    }

    /// Query all wallets (no filters)
    pub fn all() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_wallet_list_creation() {
        let query = QueryWalletList::new();
        assert!(query.bundle_hash().is_none());
        assert!(query.token_slug().is_none());
    }

    #[test]
    fn test_query_wallet_list_with_parameters() {
        let query = QueryWalletList::new()
            .with_bundle_hash("test-bundle")
            .with_token_slug("KNISH");

        assert_eq!(query.bundle_hash(), Some("test-bundle"));
        assert_eq!(query.token_slug(), Some("KNISH"));
    }

    #[test]
    fn test_with_filters() {
        let query = QueryWalletList::new().with_filters("test-bundle", "KNISH");
        assert_eq!(query.bundle_hash(), Some("test-bundle"));
        assert_eq!(query.token_slug(), Some("KNISH"));
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_bundle_hash
        let query = QueryWalletList::by_bundle_hash("test-bundle");
        assert_eq!(query.bundle_hash(), Some("test-bundle"));
        assert!(query.token_slug().is_none());

        // Test by_token_slug
        let query = QueryWalletList::by_token_slug("KNISH");
        assert!(query.bundle_hash().is_none());
        assert_eq!(query.token_slug(), Some("KNISH"));

        // Test by_bundle_and_token
        let query = QueryWalletList::by_bundle_and_token("test-bundle", "KNISH");
        assert_eq!(query.bundle_hash(), Some("test-bundle"));
        assert_eq!(query.token_slug(), Some("KNISH"));

        // Test all
        let query = QueryWalletList::all();
        assert!(query.bundle_hash().is_none());
        assert!(query.token_slug().is_none());
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryWalletList::new()
            .with_bundle_hash("test-bundle")
            .with_token_slug("KNISH");

        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["bundleHash"], json!("test-bundle"));
        assert_eq!(variables["tokenSlug"], json!("KNISH"));
    }

    #[test]
    fn test_compiled_variables_partial() {
        // Only bundle hash
        let query = QueryWalletList::new().with_bundle_hash("test-bundle");
        let variables = query.compiled_variables(None).unwrap();
        
        assert_eq!(variables["bundleHash"], json!("test-bundle"));
        assert!(!variables.as_object().unwrap().contains_key("tokenSlug"));

        // Only token slug
        let query = QueryWalletList::new().with_token_slug("KNISH");
        let variables = query.compiled_variables(None).unwrap();
        
        assert!(!variables.as_object().unwrap().contains_key("bundleHash"));
        assert_eq!(variables["tokenSlug"], json!("KNISH"));
    }

    #[test]
    fn test_compiled_variables_empty() {
        let query = QueryWalletList::new();
        let variables = query.compiled_variables(None).unwrap();
        
        let obj = variables.as_object().unwrap();
        assert!(!obj.contains_key("bundleHash"));
        assert!(!obj.contains_key("tokenSlug"));
    }

    #[test]
    fn test_query_string() {
        let query = QueryWalletList::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("Wallet( bundleHash: $bundleHash, tokenSlug: $tokenSlug )"));
        assert!(query_string.contains("address"));
        assert!(query_string.contains("bundleHash"));
        assert!(query_string.contains("token"));
        assert!(query_string.contains("tokenSlug"));
        assert!(query_string.contains("batchId"));
        assert!(query_string.contains("position"));
        assert!(query_string.contains("amount"));
        assert!(query_string.contains("characters"));
        assert!(query_string.contains("pubkey"));
        assert!(query_string.contains("createdAt"));
        assert!(query_string.contains("tokenUnits"));
        assert!(query_string.contains("tradeRates"));
        
        // Check nested structures
        assert!(query_string.contains("name"));
        assert!(query_string.contains("fungibility"));
        assert!(query_string.contains("supply"));
        assert!(query_string.contains("metas"));
    }
}