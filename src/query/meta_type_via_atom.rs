//! QueryMetaTypeViaAtom implementation
//!
//! Query for retrieving metadata through atom relationships,
//! equivalent to QueryMetaTypeViaAtom.js

use crate::query::Query;
use crate::response::{Response, ResponseMetaTypeViaAtom};
use serde_json::{json, Value};

/// Query for retrieving metadata through atom relationships
#[derive(Debug, Default)]
pub struct QueryMetaTypeViaAtom {
    /// Meta types to filter by (always array)
    meta_types: Vec<String>,
    /// Meta IDs to filter by (always array)
    meta_ids: Vec<String>,
    /// Atom values to filter by
    atom_values: Vec<String>,
    /// Cell slugs to filter by
    cell_slugs: Vec<String>,
    /// Keys to filter by
    keys: Vec<String>,
    /// Values to filter by
    values: Vec<String>,
    /// Whether to get only the latest
    latest: Option<bool>,
    /// Filter array
    filter: Vec<Value>,
    /// Query arguments for pagination
    query_args: Option<Value>,
    /// Count by parameter
    count_by: Option<String>,
}

impl QueryMetaTypeViaAtom {
    /// Create a new QueryMetaTypeViaAtom instance
    pub fn new() -> Self {
        QueryMetaTypeViaAtom::default()
    }

    /// Add a meta type
    pub fn add_meta_type(mut self, meta_type: impl Into<String>) -> Self {
        self.meta_types.push(meta_type.into());
        self
    }

    /// Set meta types (replacing any existing)
    pub fn with_meta_types(mut self, meta_types: Vec<String>) -> Self {
        self.meta_types = meta_types;
        self
    }

    /// Add a meta ID
    pub fn add_meta_id(mut self, meta_id: impl Into<String>) -> Self {
        self.meta_ids.push(meta_id.into());
        self
    }

    /// Set meta IDs (replacing any existing)
    pub fn with_meta_ids(mut self, meta_ids: Vec<String>) -> Self {
        self.meta_ids = meta_ids;
        self
    }

    /// Add an atom value
    pub fn add_atom_value(mut self, atom_value: impl Into<String>) -> Self {
        self.atom_values.push(atom_value.into());
        self
    }

    /// Set atom values (replacing any existing)
    pub fn with_atom_values(mut self, atom_values: Vec<String>) -> Self {
        self.atom_values = atom_values;
        self
    }

    /// Add a cell slug
    pub fn add_cell_slug(mut self, cell_slug: impl Into<String>) -> Self {
        self.cell_slugs.push(cell_slug.into());
        self
    }

    /// Set cell slugs (replacing any existing)
    pub fn with_cell_slugs(mut self, cell_slugs: Vec<String>) -> Self {
        self.cell_slugs = cell_slugs;
        self
    }

    /// Add a key
    pub fn add_key(mut self, key: impl Into<String>) -> Self {
        self.keys.push(key.into());
        self
    }

    /// Set keys (replacing any existing)
    pub fn with_keys(mut self, keys: Vec<String>) -> Self {
        self.keys = keys;
        self
    }

    /// Add a value
    pub fn add_value(mut self, value: impl Into<String>) -> Self {
        self.values.push(value.into());
        self
    }

    /// Set values (replacing any existing)
    pub fn with_values(mut self, values: Vec<String>) -> Self {
        self.values = values;
        self
    }

    /// Set the latest flag
    pub fn with_latest(mut self, latest: bool) -> Self {
        self.latest = Some(latest);
        self
    }

    /// Add a filter
    pub fn add_filter(mut self, filter: Value) -> Self {
        self.filter.push(filter);
        self
    }

    /// Set filters (replacing any existing)
    pub fn with_filter(mut self, filter: Vec<Value>) -> Self {
        self.filter = filter;
        self
    }

    /// Set query arguments
    pub fn with_query_args(mut self, query_args: Value) -> Self {
        self.query_args = Some(query_args);
        self
    }

    /// Set count by parameter
    pub fn with_count_by(mut self, count_by: impl Into<String>) -> Self {
        self.count_by = Some(count_by.into());
        self
    }

    /// Create variables (equivalent to createVariables in JS)
    pub fn create_variables(params: QueryMetaTypeViaAtomParams) -> Value {
        let mut variables = json!({});

        // Handle atom values
        if !params.atom_values.is_empty() {
            variables["atomValues"] = json!(params.atom_values);
        }

        // Handle keys
        if !params.keys.is_empty() {
            variables["keys"] = json!(params.keys);
        }

        // Handle values
        if !params.values.is_empty() {
            variables["values"] = json!(params.values);
        }

        // Handle meta types (convert single to array if needed)
        if let Some(meta_type) = params.meta_type {
            variables["metaTypes"] = json!([meta_type]);
        } else if !params.meta_types.is_empty() {
            variables["metaTypes"] = json!(params.meta_types);
        }

        // Handle meta IDs (convert single to array if needed)
        if let Some(meta_id) = params.meta_id {
            variables["metaIds"] = json!([meta_id]);
        } else if !params.meta_ids.is_empty() {
            variables["metaIds"] = json!(params.meta_ids);
        }

        // Handle cell slugs (convert single to array if needed)
        if let Some(cell_slug) = params.cell_slug {
            variables["cellSlugs"] = json!([cell_slug]);
        } else if !params.cell_slugs.is_empty() {
            variables["cellSlugs"] = json!(params.cell_slugs);
        }

        // Handle count by
        if let Some(count_by) = params.count_by {
            variables["countBy"] = json!(count_by);
        }

        // Handle filter
        let mut filter = params.filter;
        
        // Add key/value filter if both are provided
        if let (Some(key), Some(value)) = (params.key, params.value) {
            filter.push(json!({
                "key": key,
                "value": value,
                "comparison": "="
            }));
        }
        
        if !filter.is_empty() {
            variables["filter"] = json!(filter);
        }

        // Set latest flag (defaults to false)
        variables["latest"] = json!(params.latest.unwrap_or(false));

        // Handle query args
        if let Some(mut query_args) = params.query_args {
            // Handle limit = 0 case (convert to "*")
            if let Some(limit) = query_args.get("limit") {
                if limit.as_i64() == Some(0) {
                    query_args["limit"] = json!("*");
                }
            }
            variables["queryArgs"] = query_args;
        }

        variables
    }
}

#[async_trait::async_trait]
impl Query for QueryMetaTypeViaAtom {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        r#"query ($metaTypes: [String!], $metaIds: [String!], $values: [String!], $keys: [String!], $latest: Boolean, $filter: [MetaFilter!], $queryArgs: QueryArgs, $countBy: String, $atomValues: [String!], $cellSlugs: [String!] ) {
          MetaTypeViaAtom(
            metaTypes: $metaTypes
            metaIds: $metaIds
            atomValues: $atomValues
            cellSlugs: $cellSlugs
            filter: $filter,
            latest: $latest,
            queryArgs: $queryArgs
            countBy: $countBy
          ) {
            metaType,
            instanceCount {
              key,
              value
            },
            instances {
              metaType,
              metaId,
              createdAt,
              metas( values: $values, keys: $keys ) {
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
            let params = QueryMetaTypeViaAtomParams {
                meta_type: None,
                meta_types: self.meta_types.clone(),
                meta_id: None,
                meta_ids: self.meta_ids.clone(),
                key: None,
                value: None,
                keys: self.keys.clone(),
                values: self.values.clone(),
                atom_values: self.atom_values.clone(),
                cell_slug: None,
                cell_slugs: self.cell_slugs.clone(),
                latest: self.latest,
                filter: self.filter.clone(),
                query_args: self.query_args.clone(),
                count_by: self.count_by.clone(),
            };
            Some(Self::create_variables(params))
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseMetaTypeViaAtom::new(json, None).expect("Failed to create ResponseMetaTypeViaAtom"))
    }
}

/// Parameters for createVariables method
#[derive(Default)]
pub struct QueryMetaTypeViaAtomParams {
    pub meta_type: Option<String>,
    pub meta_types: Vec<String>,
    pub meta_id: Option<String>,
    pub meta_ids: Vec<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    pub keys: Vec<String>,
    pub values: Vec<String>,
    pub atom_values: Vec<String>,
    pub cell_slug: Option<String>,
    pub cell_slugs: Vec<String>,
    pub latest: Option<bool>,
    pub filter: Vec<Value>,
    pub query_args: Option<Value>,
    pub count_by: Option<String>,
}

/// Convenience methods for common usage patterns
impl QueryMetaTypeViaAtom {
    /// Query by meta type
    pub fn by_meta_type(meta_type: impl Into<String>) -> Self {
        Self::new().add_meta_type(meta_type)
    }

    /// Query by meta types
    pub fn by_meta_types(meta_types: Vec<String>) -> Self {
        Self::new().with_meta_types(meta_types)
    }

    /// Query by atom values
    pub fn by_atom_values(atom_values: Vec<String>) -> Self {
        Self::new().with_atom_values(atom_values)
    }

    /// Query latest only
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
    fn test_query_meta_type_via_atom_creation() {
        let query = QueryMetaTypeViaAtom::new();
        assert!(query.meta_types.is_empty());
        assert!(query.meta_ids.is_empty());
        assert!(query.atom_values.is_empty());
        assert!(query.latest.is_none());
    }

    #[test]
    fn test_query_meta_type_via_atom_with_values() {
        let query = QueryMetaTypeViaAtom::new()
            .add_meta_type("user")
            .add_meta_id("123")
            .add_atom_value("test-value")
            .with_latest(true);

        assert_eq!(query.meta_types.len(), 1);
        assert_eq!(query.meta_ids.len(), 1);
        assert_eq!(query.atom_values.len(), 1);
        assert_eq!(query.latest, Some(true));
    }

    #[test]
    fn test_create_variables_single_values() {
        let params = QueryMetaTypeViaAtomParams {
            meta_type: Some("user".to_string()),
            meta_id: Some("123".to_string()),
            key: Some("name".to_string()),
            value: Some("John".to_string()),
            latest: Some(true),
            ..Default::default()
        };

        let variables = QueryMetaTypeViaAtom::create_variables(params);
        assert_eq!(variables["metaTypes"], json!(["user"]));
        assert_eq!(variables["metaIds"], json!(["123"]));
        assert_eq!(variables["latest"], json!(true));
        
        // Check that key/value created a filter
        let filter = variables["filter"].as_array().unwrap();
        assert_eq!(filter.len(), 1);
        assert_eq!(filter[0]["key"], json!("name"));
        assert_eq!(filter[0]["value"], json!("John"));
        assert_eq!(filter[0]["comparison"], json!("="));
    }

    #[test]
    fn test_create_variables_multiple_values() {
        let params = QueryMetaTypeViaAtomParams {
            meta_types: vec!["user".to_string(), "admin".to_string()],
            meta_ids: vec!["123".to_string(), "456".to_string()],
            atom_values: vec!["val1".to_string(), "val2".to_string()],
            ..Default::default()
        };

        let variables = QueryMetaTypeViaAtom::create_variables(params);
        assert_eq!(variables["metaTypes"], json!(["user", "admin"]));
        assert_eq!(variables["metaIds"], json!(["123", "456"]));
        assert_eq!(variables["atomValues"], json!(["val1", "val2"]));
    }

    #[test]
    fn test_create_variables_with_limit_zero() {
        let params = QueryMetaTypeViaAtomParams {
            query_args: Some(json!({ "limit": 0 })),
            ..Default::default()
        };

        let variables = QueryMetaTypeViaAtom::create_variables(params);
        assert_eq!(variables["queryArgs"]["limit"], json!("*"));
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_meta_type
        let query = QueryMetaTypeViaAtom::by_meta_type("user");
        assert_eq!(query.meta_types.len(), 1);
        assert_eq!(query.meta_types[0], "user");

        // Test by_atom_values
        let query = QueryMetaTypeViaAtom::by_atom_values(vec!["val1".to_string(), "val2".to_string()]);
        assert_eq!(query.atom_values.len(), 2);

        // Test latest
        let query = QueryMetaTypeViaAtom::latest();
        assert_eq!(query.latest, Some(true));

        // Test paginated
        let query = QueryMetaTypeViaAtom::paginated(2, 10);
        assert!(query.query_args.is_some());
    }

    #[test]
    fn test_query_string() {
        let query = QueryMetaTypeViaAtom::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("MetaTypeViaAtom("));
        assert!(query_string.contains("metaTypes: $metaTypes"));
        assert!(query_string.contains("atomValues: $atomValues"));
        assert!(query_string.contains("cellSlugs: $cellSlugs"));
        assert!(query_string.contains("instanceCount"));
        assert!(query_string.contains("instances"));
        assert!(query_string.contains("metas( values: $values, keys: $keys )"));
        assert!(query_string.contains("paginatorInfo"));
    }
}