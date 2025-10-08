//! QueryToken implementation
//!
//! Query for getting token information,
//! equivalent to QueryToken.js

use crate::query::Query;
use crate::response::{Response, BaseResponse}; // No specific ResponseToken - uses BaseResponse
use serde_json::{json, Value};

/// Query for getting token information
pub struct QueryToken {
    /// Single token slug to query
    slug: Option<String>,
    /// Array of token slugs to query
    slugs: Vec<String>,
    /// Limit for results
    limit: Option<i32>,
    /// Order for results
    order: Option<String>,
}

impl QueryToken {
    /// Create a new QueryToken instance
    pub fn new() -> Self {
        QueryToken {
            slug: None,
            slugs: Vec::new(),
            limit: None,
            order: None,
        }
    }

    /// Set the token slug parameter
    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Add a token slug to the slugs array
    pub fn add_slug(mut self, slug: impl Into<String>) -> Self {
        self.slugs.push(slug.into());
        self
    }

    /// Set multiple token slugs
    pub fn with_slugs(mut self, slugs: Vec<String>) -> Self {
        self.slugs = slugs;
        self
    }

    /// Add multiple token slugs
    pub fn add_slugs(mut self, slugs: Vec<String>) -> Self {
        self.slugs.extend(slugs);
        self
    }

    /// Set the limit parameter
    pub fn with_limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the order parameter
    pub fn with_order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    /// Get the slug
    pub fn slug(&self) -> Option<&str> {
        self.slug.as_deref()
    }

    /// Get the slugs array
    pub fn slugs(&self) -> &[String] {
        &self.slugs
    }

    /// Get the limit
    pub fn limit(&self) -> Option<i32> {
        self.limit
    }

    /// Get the order
    pub fn order(&self) -> Option<&str> {
        self.order.as_deref()
    }
}

impl Default for QueryToken {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for QueryToken {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query( $slug: String, $slugs: [ String! ], $limit: Int, $order: String ) {
          Token( slug: $slug, slugs: $slugs, limit: $limit, order: $order ) {
            slug,
            name,
            fungibility,
            supply,
            decimals,
            amount,
            icon
          }
        }"#
    }

    /// Compile variables for the query (equivalent to compiledVariables in JS)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        if let Some(provided_vars) = variables {
            Some(provided_vars)
        } else {
            let mut vars = json!({});

            if let Some(ref slug) = self.slug {
                vars["slug"] = json!(slug);
            }
            if !self.slugs.is_empty() {
                vars["slugs"] = json!(self.slugs);
            }
            if let Some(limit) = self.limit {
                vars["limit"] = json!(limit);
            }
            if let Some(ref order) = self.order {
                vars["order"] = json!(order);
            }

            Some(vars)
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        // Equivalent to dataKey: 'data.Token' in JS
        Box::new(BaseResponse::new(json).expect("Failed to create BaseResponse").with_data_key("data.Token"))
    }
}

/// Convenience methods for common usage patterns
impl QueryToken {
    /// Query token by slug (most common pattern)
    pub fn by_slug(slug: impl Into<String>) -> Self {
        Self::new().with_slug(slug)
    }

    /// Query multiple tokens by slugs
    pub fn by_slugs(slugs: Vec<String>) -> Self {
        Self::new().with_slugs(slugs)
    }

    /// Query all tokens with a limit
    pub fn all_with_limit(limit: i32) -> Self {
        Self::new().with_limit(limit)
    }

    /// Query tokens with specific ordering
    pub fn with_ordering(order: impl Into<String>) -> Self {
        Self::new().with_order(order)
    }

    /// Query limited tokens with ordering
    pub fn limited_ordered(limit: i32, order: impl Into<String>) -> Self {
        Self::new().with_limit(limit).with_order(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_token_creation() {
        let query = QueryToken::new();
        assert!(query.slug().is_none());
        assert!(query.slugs().is_empty());
        assert!(query.limit().is_none());
        assert!(query.order().is_none());
    }

    #[test]
    fn test_query_token_with_parameters() {
        let query = QueryToken::new()
            .with_slug("KNISH")
            .with_limit(10)
            .with_order("createdAt");

        assert_eq!(query.slug(), Some("KNISH"));
        assert_eq!(query.limit(), Some(10));
        assert_eq!(query.order(), Some("createdAt"));
    }

    #[test]
    fn test_query_token_with_slugs() {
        let slugs = vec!["KNISH".to_string(), "TOKEN2".to_string()];
        let query = QueryToken::new().with_slugs(slugs.clone());
        assert_eq!(query.slugs(), slugs.as_slice());
    }

    #[test]
    fn test_add_slug() {
        let query = QueryToken::new()
            .add_slug("KNISH")
            .add_slug("TOKEN2");

        assert_eq!(query.slugs().len(), 2);
        assert_eq!(query.slugs()[0], "KNISH");
        assert_eq!(query.slugs()[1], "TOKEN2");
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_slug
        let query = QueryToken::by_slug("KNISH");
        assert_eq!(query.slug(), Some("KNISH"));

        // Test by_slugs
        let slugs = vec!["KNISH".to_string(), "TOKEN2".to_string()];
        let query = QueryToken::by_slugs(slugs.clone());
        assert_eq!(query.slugs(), slugs.as_slice());

        // Test all_with_limit
        let query = QueryToken::all_with_limit(100);
        assert_eq!(query.limit(), Some(100));

        // Test with_ordering
        let query = QueryToken::with_ordering("name");
        assert_eq!(query.order(), Some("name"));

        // Test limited_ordered
        let query = QueryToken::limited_ordered(50, "createdAt");
        assert_eq!(query.limit(), Some(50));
        assert_eq!(query.order(), Some("createdAt"));
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryToken::new()
            .with_slug("KNISH")
            .with_limit(10)
            .with_order("createdAt");

        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["slug"], json!("KNISH"));
        assert_eq!(variables["limit"], json!(10));
        assert_eq!(variables["order"], json!("createdAt"));
    }

    #[test]
    fn test_compiled_variables_with_slugs() {
        let slugs = vec!["KNISH".to_string(), "TOKEN2".to_string()];
        let query = QueryToken::new().with_slugs(slugs.clone());

        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["slugs"], json!(slugs));
    }

    #[test]
    fn test_query_string() {
        let query = QueryToken::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("Token( slug: $slug, slugs: $slugs, limit: $limit, order: $order )"));
        assert!(query_string.contains("slug"));
        assert!(query_string.contains("name"));
        assert!(query_string.contains("fungibility"));
        assert!(query_string.contains("supply"));
        assert!(query_string.contains("decimals"));
        assert!(query_string.contains("amount"));
        assert!(query_string.contains("icon"));
    }
}