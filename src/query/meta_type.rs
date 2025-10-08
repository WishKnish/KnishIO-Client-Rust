//! QueryMetaType implementation
//!
//! Query for retrieving Meta Asset information,
//! equivalent to QueryMetaType.js

use crate::query::Query;
use crate::response::{Response, ResponseMetaType};
use serde_json::{json, Value};

/// Query for retrieving Meta Asset information
#[derive(Debug, Default)]
pub struct QueryMetaType {
    /// Single meta type or array of meta types
    meta_type: Option<MetaTypeValue>,
    /// Single meta ID or array of meta IDs
    meta_id: Option<MetaIdValue>,
    /// Single key or array of keys
    key: Option<KeyValue>,
    /// Single value or array of values
    value: Option<ValueValue>,
    /// Whether to get only the latest metas
    latest: Option<bool>,
    /// Filter object for meta queries
    filter: Option<Value>,
    /// Query arguments for pagination etc.
    query_args: Option<Value>,
    /// Count parameter
    count: Option<String>,
    /// Count by parameter
    count_by: Option<String>,
    /// Cell slug parameter
    cell_slug: Option<String>,
}

/// Enum to handle single value or array of values for meta_type
#[derive(Debug, Clone)]
pub enum MetaTypeValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Enum to handle single value or array of values for meta_id
#[derive(Debug, Clone)]
pub enum MetaIdValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Enum to handle single value or array of values for key
#[derive(Debug, Clone)]
pub enum KeyValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Enum to handle single value or array of values for value
#[derive(Debug, Clone)]
pub enum ValueValue {
    Single(String),
    Multiple(Vec<String>),
}

impl QueryMetaType {
    /// Create a new QueryMetaType instance
    pub fn new() -> Self {
        QueryMetaType::default()
    }

    /// Set a single meta type
    pub fn with_meta_type(mut self, meta_type: impl Into<String>) -> Self {
        self.meta_type = Some(MetaTypeValue::Single(meta_type.into()));
        self
    }

    /// Set multiple meta types
    pub fn with_meta_types(mut self, meta_types: Vec<String>) -> Self {
        self.meta_type = Some(MetaTypeValue::Multiple(meta_types));
        self
    }

    /// Set a single meta ID
    pub fn with_meta_id(mut self, meta_id: impl Into<String>) -> Self {
        self.meta_id = Some(MetaIdValue::Single(meta_id.into()));
        self
    }

    /// Set multiple meta IDs
    pub fn with_meta_ids(mut self, meta_ids: Vec<String>) -> Self {
        self.meta_id = Some(MetaIdValue::Multiple(meta_ids));
        self
    }

    /// Set a single key
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(KeyValue::Single(key.into()));
        self
    }

    /// Set multiple keys
    pub fn with_keys(mut self, keys: Vec<String>) -> Self {
        self.key = Some(KeyValue::Multiple(keys));
        self
    }

    /// Set a single value
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(ValueValue::Single(value.into()));
        self
    }

    /// Set multiple values
    pub fn with_values(mut self, values: Vec<String>) -> Self {
        self.value = Some(ValueValue::Multiple(values));
        self
    }

    /// Set the latest flag
    pub fn with_latest(mut self, latest: bool) -> Self {
        self.latest = Some(latest);
        self
    }

    /// Set the filter
    pub fn with_filter(mut self, filter: Value) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Set the query arguments
    pub fn with_query_args(mut self, query_args: Value) -> Self {
        self.query_args = Some(query_args);
        self
    }

    /// Set the count parameter
    pub fn with_count(mut self, count: impl Into<String>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Set the count by parameter
    pub fn with_count_by(mut self, count_by: impl Into<String>) -> Self {
        self.count_by = Some(count_by.into());
        self
    }

    /// Set the cell slug parameter
    pub fn with_cell_slug(mut self, cell_slug: impl Into<String>) -> Self {
        self.cell_slug = Some(cell_slug.into());
        self
    }

    /// Create variables (equivalent to createVariables in JS)
    pub fn create_variables(params: QueryMetaTypeParams) -> Value {
        let mut variables = json!({});

        // Handle meta_type (single or array)
        if let Some(meta_type) = params.meta_type {
            match meta_type {
                MetaTypeValue::Single(val) => variables["metaType"] = json!(val),
                MetaTypeValue::Multiple(vals) => variables["metaTypes"] = json!(vals),
            }
        }

        // Handle meta_id (single or array)
        if let Some(meta_id) = params.meta_id {
            match meta_id {
                MetaIdValue::Single(val) => variables["metaId"] = json!(val),
                MetaIdValue::Multiple(vals) => variables["metaIds"] = json!(vals),
            }
        }

        // Handle key (single or array)
        if let Some(key) = params.key {
            match key {
                KeyValue::Single(val) => variables["key"] = json!(val),
                KeyValue::Multiple(vals) => variables["keys"] = json!(vals),
            }
        }

        // Handle value (single or array)
        if let Some(value) = params.value {
            match value {
                ValueValue::Single(val) => variables["value"] = json!(val),
                ValueValue::Multiple(vals) => variables["values"] = json!(vals),
            }
        }

        // Set latest flag (defaults to false)
        variables["latest"] = json!(params.latest.unwrap_or(false));

        // Handle optional parameters
        if let Some(filter) = params.filter {
            variables["filter"] = filter;
        }

        if let Some(mut query_args) = params.query_args {
            // Handle limit = 0 case (convert to "*")
            if let Some(limit) = query_args.get("limit") {
                if limit.as_i64() == Some(0) {
                    query_args["limit"] = json!("*");
                }
            }
            variables["queryArgs"] = query_args;
        }

        if let Some(count) = params.count {
            variables["count"] = json!(count);
        }

        if let Some(count_by) = params.count_by {
            variables["countBy"] = json!(count_by);
        }

        if let Some(cell_slug) = params.cell_slug {
            variables["cellSlug"] = json!(cell_slug);
        }

        variables
    }
}

#[async_trait::async_trait]
impl Query for QueryMetaType {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query( $metaType: String, $metaTypes: [ String! ], $metaId: String, $metaIds: [ String! ], $key: String, $keys: [ String! ], $value: String, $values: [ String! ], $count: String, $latest: Boolean, $filter: [ MetaFilter! ], $queryArgs: QueryArgs, $countBy: String, $cellSlug: String ) {
          MetaType( metaType: $metaType, metaTypes: $metaTypes, metaId: $metaId, metaIds: $metaIds, key: $key, keys: $keys, value: $value, values: $values, count: $count, filter: $filter, queryArgs: $queryArgs, countBy: $countBy, cellSlug: $cellSlug ) {
            metaType,
            instanceCount {
              key,
              value
            },
            instances {
              metaType,
              metaId,
              createdAt,
              metas(latest:$latest) {
                molecularHash,
                position,
                key,
                value,
                createdAt
              }
            },
            paginatorInfo {
              currentPage,
              total
            }
          }
        }"#
    }

    /// Compile variables for the query (equivalent to compiledVariables in JS)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        if let Some(provided_vars) = variables {
            Some(provided_vars)
        } else {
            // Use create_variables with the instance parameters
            let params = QueryMetaTypeParams {
                meta_type: self.meta_type.clone(),
                meta_id: self.meta_id.clone(),
                key: self.key.clone(),
                value: self.value.clone(),
                latest: self.latest,
                filter: self.filter.clone(),
                query_args: self.query_args.clone(),
                count: self.count.clone(),
                count_by: self.count_by.clone(),
                cell_slug: self.cell_slug.clone(),
            };
            Some(Self::create_variables(params))
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseMetaType::new(json, None).expect("Failed to create ResponseMetaType"))
    }
}

/// Parameters for createVariables method
#[derive(Default)]
pub struct QueryMetaTypeParams {
    pub meta_type: Option<MetaTypeValue>,
    pub meta_id: Option<MetaIdValue>,
    pub key: Option<KeyValue>,
    pub value: Option<ValueValue>,
    pub latest: Option<bool>,
    pub filter: Option<Value>,
    pub query_args: Option<Value>,
    pub count: Option<String>,
    pub count_by: Option<String>,
    pub cell_slug: Option<String>,
}

/// Convenience methods for common usage patterns
impl QueryMetaType {
    /// Query by meta type
    pub fn by_meta_type(meta_type: impl Into<String>) -> Self {
        Self::new().with_meta_type(meta_type)
    }

    /// Query by meta types (multiple)
    pub fn by_meta_types(meta_types: Vec<String>) -> Self {
        Self::new().with_meta_types(meta_types)
    }

    /// Query by meta ID
    pub fn by_meta_id(meta_id: impl Into<String>) -> Self {
        Self::new().with_meta_id(meta_id)
    }

    /// Query by meta type and ID
    pub fn by_meta(meta_type: impl Into<String>, meta_id: impl Into<String>) -> Self {
        Self::new()
            .with_meta_type(meta_type)
            .with_meta_id(meta_id)
    }

    /// Query latest metas only
    pub fn latest() -> Self {
        Self::new().with_latest(true)
    }

    /// Query with pagination
    pub fn paginated(page: i32, limit: i32) -> Self {
        Self::new().with_query_args(json!({
            "page": page,
            "limit": limit
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_meta_type_creation() {
        let query = QueryMetaType::new();
        assert!(query.meta_type.is_none());
        assert!(query.meta_id.is_none());
        assert!(query.latest.is_none());
    }

    #[test]
    fn test_query_meta_type_with_single_values() {
        let query = QueryMetaType::new()
            .with_meta_type("user")
            .with_meta_id("123")
            .with_key("name")
            .with_value("John");

        assert!(matches!(query.meta_type, Some(MetaTypeValue::Single(_))));
        assert!(matches!(query.meta_id, Some(MetaIdValue::Single(_))));
        assert!(matches!(query.key, Some(KeyValue::Single(_))));
        assert!(matches!(query.value, Some(ValueValue::Single(_))));
    }

    #[test]
    fn test_query_meta_type_with_multiple_values() {
        let meta_types = vec!["user".to_string(), "admin".to_string()];
        let meta_ids = vec!["123".to_string(), "456".to_string()];
        
        let query = QueryMetaType::new()
            .with_meta_types(meta_types.clone())
            .with_meta_ids(meta_ids.clone());

        assert!(matches!(query.meta_type, Some(MetaTypeValue::Multiple(_))));
        assert!(matches!(query.meta_id, Some(MetaIdValue::Multiple(_))));
    }

    #[test]
    fn test_create_variables_single_values() {
        let params = QueryMetaTypeParams {
            meta_type: Some(MetaTypeValue::Single("user".to_string())),
            meta_id: Some(MetaIdValue::Single("123".to_string())),
            latest: Some(true),
            ..Default::default()
        };

        let variables = QueryMetaType::create_variables(params);
        assert_eq!(variables["metaType"], json!("user"));
        assert_eq!(variables["metaId"], json!("123"));
        assert_eq!(variables["latest"], json!(true));
    }

    #[test]
    fn test_create_variables_multiple_values() {
        let params = QueryMetaTypeParams {
            meta_type: Some(MetaTypeValue::Multiple(vec!["user".to_string(), "admin".to_string()])),
            meta_id: Some(MetaIdValue::Multiple(vec!["123".to_string(), "456".to_string()])),
            ..Default::default()
        };

        let variables = QueryMetaType::create_variables(params);
        assert_eq!(variables["metaTypes"], json!(["user", "admin"]));
        assert_eq!(variables["metaIds"], json!(["123", "456"]));
    }

    #[test]
    fn test_create_variables_with_limit_zero() {
        let params = QueryMetaTypeParams {
            query_args: Some(json!({ "limit": 0 })),
            ..Default::default()
        };

        let variables = QueryMetaType::create_variables(params);
        assert_eq!(variables["queryArgs"]["limit"], json!("*"));
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_meta_type
        let query = QueryMetaType::by_meta_type("user");
        assert!(matches!(query.meta_type, Some(MetaTypeValue::Single(_))));

        // Test by_meta_types
        let query = QueryMetaType::by_meta_types(vec!["user".to_string(), "admin".to_string()]);
        assert!(matches!(query.meta_type, Some(MetaTypeValue::Multiple(_))));

        // Test latest
        let query = QueryMetaType::latest();
        assert_eq!(query.latest, Some(true));

        // Test paginated
        let query = QueryMetaType::paginated(2, 10);
        assert!(query.query_args.is_some());
    }

    #[test]
    fn test_query_string() {
        let query = QueryMetaType::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("MetaType("));
        assert!(query_string.contains("metaType: $metaType"));
        assert!(query_string.contains("metaTypes: $metaTypes"));
        assert!(query_string.contains("instanceCount"));
        assert!(query_string.contains("instances"));
        assert!(query_string.contains("metas(latest:$latest)"));
        assert!(query_string.contains("paginatorInfo"));
    }
}