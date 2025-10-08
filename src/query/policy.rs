//! QueryPolicy implementation
//!
//! Query for getting policy information,
//! equivalent to QueryPolicy.js

use crate::query::Query;
use crate::response::{Response, ResponsePolicy};
use serde_json::{json, Value};

/// Query for getting policy information
pub struct QueryPolicy {
    /// Meta type to filter by
    meta_type: Option<String>,
    /// Meta ID to filter by
    meta_id: Option<String>,
}

impl QueryPolicy {
    /// Create a new QueryPolicy instance
    pub fn new() -> Self {
        QueryPolicy {
            meta_type: None,
            meta_id: None,
        }
    }

    /// Set the meta type parameter
    pub fn with_meta_type(mut self, meta_type: impl Into<String>) -> Self {
        self.meta_type = Some(meta_type.into());
        self
    }

    /// Set the meta ID parameter
    pub fn with_meta_id(mut self, meta_id: impl Into<String>) -> Self {
        self.meta_id = Some(meta_id.into());
        self
    }

    /// Set both meta type and meta ID
    pub fn with_meta(mut self, meta_type: impl Into<String>, meta_id: impl Into<String>) -> Self {
        self.meta_type = Some(meta_type.into());
        self.meta_id = Some(meta_id.into());
        self
    }

    /// Get the meta type
    pub fn meta_type(&self) -> Option<&str> {
        self.meta_type.as_deref()
    }

    /// Get the meta ID
    pub fn meta_id(&self) -> Option<&str> {
        self.meta_id.as_deref()
    }
}

impl Default for QueryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for QueryPolicy {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query( $metaType: String, $metaId: String ) {
          Policy( metaType: $metaType, metaId: $metaId ) {
            molecularHash,
            position,
            metaType,
            metaId,
            conditions,
            callback,
            rule,
            createdAt
          }
        }"#
    }

    /// Compile variables for the query (equivalent to compiledVariables in JS)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        if let Some(provided_vars) = variables {
            Some(provided_vars)
        } else {
            let mut vars = json!({});

            if let Some(ref meta_type) = self.meta_type {
                vars["metaType"] = json!(meta_type);
            }
            if let Some(ref meta_id) = self.meta_id {
                vars["metaId"] = json!(meta_id);
            }

            Some(vars)
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponsePolicy::new(json, None).expect("Failed to create ResponsePolicy"))
    }
}

/// Convenience methods for common usage patterns
impl QueryPolicy {
    /// Query policy by meta type
    pub fn by_meta_type(meta_type: impl Into<String>) -> Self {
        Self::new().with_meta_type(meta_type)
    }

    /// Query policy by meta ID
    pub fn by_meta_id(meta_id: impl Into<String>) -> Self {
        Self::new().with_meta_id(meta_id)
    }

    /// Query policy by both meta type and meta ID
    pub fn by_meta(meta_type: impl Into<String>, meta_id: impl Into<String>) -> Self {
        Self::new().with_meta(meta_type, meta_id)
    }

    /// Query all policies (no filters)
    pub fn all() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_policy_creation() {
        let query = QueryPolicy::new();
        assert!(query.meta_type().is_none());
        assert!(query.meta_id().is_none());
    }

    #[test]
    fn test_query_policy_with_parameters() {
        let query = QueryPolicy::new()
            .with_meta_type("user")
            .with_meta_id("123");

        assert_eq!(query.meta_type(), Some("user"));
        assert_eq!(query.meta_id(), Some("123"));
    }

    #[test]
    fn test_with_meta() {
        let query = QueryPolicy::new().with_meta("user", "456");
        assert_eq!(query.meta_type(), Some("user"));
        assert_eq!(query.meta_id(), Some("456"));
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_meta_type
        let query = QueryPolicy::by_meta_type("user");
        assert_eq!(query.meta_type(), Some("user"));
        assert!(query.meta_id().is_none());

        // Test by_meta_id
        let query = QueryPolicy::by_meta_id("123");
        assert!(query.meta_type().is_none());
        assert_eq!(query.meta_id(), Some("123"));

        // Test by_meta
        let query = QueryPolicy::by_meta("user", "456");
        assert_eq!(query.meta_type(), Some("user"));
        assert_eq!(query.meta_id(), Some("456"));

        // Test all
        let query = QueryPolicy::all();
        assert!(query.meta_type().is_none());
        assert!(query.meta_id().is_none());
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryPolicy::new()
            .with_meta_type("user")
            .with_meta_id("123");

        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["metaType"], json!("user"));
        assert_eq!(variables["metaId"], json!("123"));
    }

    #[test]
    fn test_compiled_variables_partial() {
        let query = QueryPolicy::new().with_meta_type("user");
        let variables = query.compiled_variables(None).unwrap();
        
        assert_eq!(variables["metaType"], json!("user"));
        assert!(!variables.as_object().unwrap().contains_key("metaId"));
    }

    #[test]
    fn test_compiled_variables_empty() {
        let query = QueryPolicy::new();
        let variables = query.compiled_variables(None).unwrap();
        
        let obj = variables.as_object().unwrap();
        assert!(!obj.contains_key("metaType"));
        assert!(!obj.contains_key("metaId"));
    }

    #[test]
    fn test_query_string() {
        let query = QueryPolicy::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("Policy( metaType: $metaType, metaId: $metaId )"));
        assert!(query_string.contains("molecularHash"));
        assert!(query_string.contains("position"));
        assert!(query_string.contains("metaType"));
        assert!(query_string.contains("metaId"));
        assert!(query_string.contains("conditions"));
        assert!(query_string.contains("callback"));
        assert!(query_string.contains("rule"));
        assert!(query_string.contains("createdAt"));
    }
}