//! QueryActiveSession implementation
//!
//! Query for retrieving a list of active User Sessions,
//! equivalent to QueryActiveSession.js

use crate::query::Query;
use crate::response::{Response, ResponseQueryActiveSession};
use serde_json::{json, Value};

/// Query for retrieving a list of active User Sessions
pub struct QueryActiveSession {
    /// Optional bundle hash to filter by
    bundle_hash: Option<String>,
    /// Optional meta type to filter by
    meta_type: Option<String>,
    /// Optional meta ID to filter by
    meta_id: Option<String>,
}

impl QueryActiveSession {
    /// Create a new QueryActiveSession instance
    pub fn new() -> Self {
        QueryActiveSession {
            bundle_hash: None,
            meta_type: None,
            meta_id: None,
        }
    }

    /// Set the bundle hash parameter
    pub fn with_bundle_hash(mut self, bundle_hash: impl Into<String>) -> Self {
        self.bundle_hash = Some(bundle_hash.into());
        self
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

    /// Set meta type and meta ID together
    pub fn with_meta(mut self, meta_type: impl Into<String>, meta_id: impl Into<String>) -> Self {
        self.meta_type = Some(meta_type.into());
        self.meta_id = Some(meta_id.into());
        self
    }

    /// Get the bundle hash
    pub fn bundle_hash(&self) -> Option<&str> {
        self.bundle_hash.as_deref()
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

impl Default for QueryActiveSession {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for QueryActiveSession {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query ActiveUserQuery ($bundleHash:String, $metaType: String, $metaId: String) {
          ActiveUser (bundleHash: $bundleHash, metaType: $metaType, metaId: $metaId) {
            bundleHash,
            metaType,
            metaId,
            jsonData,
            createdAt,
            updatedAt
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
        Box::new(ResponseQueryActiveSession::new(json))
    }
}

/// Convenience methods for common usage patterns
impl QueryActiveSession {
    /// Query active sessions by bundle hash
    pub fn by_bundle_hash(bundle_hash: impl Into<String>) -> Self {
        Self::new().with_bundle_hash(bundle_hash)
    }

    /// Query active sessions by meta type
    pub fn by_meta_type(meta_type: impl Into<String>) -> Self {
        Self::new().with_meta_type(meta_type)
    }

    /// Query active sessions by meta ID
    pub fn by_meta_id(meta_id: impl Into<String>) -> Self {
        Self::new().with_meta_id(meta_id)
    }

    /// Query active sessions by meta type and ID
    pub fn by_meta(meta_type: impl Into<String>, meta_id: impl Into<String>) -> Self {
        Self::new().with_meta(meta_type, meta_id)
    }

    /// Query all active sessions (no filters)
    pub fn all() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_active_session_creation() {
        let query = QueryActiveSession::new();
        assert!(query.bundle_hash().is_none());
        assert!(query.meta_type().is_none());
        assert!(query.meta_id().is_none());
    }

    #[test]
    fn test_query_active_session_with_parameters() {
        let query = QueryActiveSession::new()
            .with_bundle_hash("test-bundle")
            .with_meta_type("user")
            .with_meta_id("123");

        assert_eq!(query.bundle_hash(), Some("test-bundle"));
        assert_eq!(query.meta_type(), Some("user"));
        assert_eq!(query.meta_id(), Some("123"));
    }

    #[test]
    fn test_with_meta() {
        let query = QueryActiveSession::new().with_meta("session", "456");
        assert_eq!(query.meta_type(), Some("session"));
        assert_eq!(query.meta_id(), Some("456"));
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_bundle_hash
        let query = QueryActiveSession::by_bundle_hash("test-bundle");
        assert_eq!(query.bundle_hash(), Some("test-bundle"));

        // Test by_meta_type
        let query = QueryActiveSession::by_meta_type("user");
        assert_eq!(query.meta_type(), Some("user"));

        // Test by_meta_id
        let query = QueryActiveSession::by_meta_id("123");
        assert_eq!(query.meta_id(), Some("123"));

        // Test by_meta
        let query = QueryActiveSession::by_meta("user", "456");
        assert_eq!(query.meta_type(), Some("user"));
        assert_eq!(query.meta_id(), Some("456"));

        // Test all
        let query = QueryActiveSession::all();
        assert!(query.bundle_hash().is_none());
        assert!(query.meta_type().is_none());
        assert!(query.meta_id().is_none());
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryActiveSession::new()
            .with_bundle_hash("test-bundle")
            .with_meta_type("user")
            .with_meta_id("123");

        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["bundleHash"], json!("test-bundle"));
        assert_eq!(variables["metaType"], json!("user"));
        assert_eq!(variables["metaId"], json!("123"));
    }

    #[test]
    fn test_compiled_variables_partial() {
        let query = QueryActiveSession::new().with_bundle_hash("test-bundle");
        let variables = query.compiled_variables(None).unwrap();
        
        assert_eq!(variables["bundleHash"], json!("test-bundle"));
        assert!(!variables.as_object().unwrap().contains_key("metaType"));
        assert!(!variables.as_object().unwrap().contains_key("metaId"));
    }

    #[test]
    fn test_query_string() {
        let query = QueryActiveSession::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("ActiveUserQuery"));
        assert!(query_string.contains("ActiveUser"));
        assert!(query_string.contains("bundleHash"));
        assert!(query_string.contains("metaType"));
        assert!(query_string.contains("metaId"));
        assert!(query_string.contains("jsonData"));
        assert!(query_string.contains("createdAt"));
        assert!(query_string.contains("updatedAt"));
    }
}