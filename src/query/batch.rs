//! QueryBatch implementation
//!
//! Query for retrieving batch information,
//! equivalent to QueryBatch.js

use crate::query::Query;
use crate::response::{Response, ResponseMetaBatch};
use serde_json::{json, Value};

/// Query for retrieving batch information
pub struct QueryBatch {
    /// Optional batch ID to query
    batch_id: Option<String>,
}

impl QueryBatch {
    /// Create a new QueryBatch instance
    pub fn new() -> Self {
        QueryBatch { batch_id: None }
    }

    /// Create a new QueryBatch with batch ID
    pub fn with_batch_id(batch_id: impl Into<String>) -> Self {
        QueryBatch {
            batch_id: Some(batch_id.into()),
        }
    }

    /// Set the batch ID parameter
    pub fn set_batch_id(&mut self, batch_id: impl Into<String>) {
        self.batch_id = Some(batch_id.into());
    }

    /// Get the batch ID
    pub fn batch_id(&self) -> Option<&str> {
        self.batch_id.as_deref()
    }

}

impl Default for QueryBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Query for QueryBatch {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    fn get_query(&self) -> &str {
        // Note: In the actual implementation, we would need to substitute the fields
        // For now, including the full query as a static string
        r#"query( $batchId: String ) {
          Batch( batchId: $batchId ) {
            batchId,
            molecularHash,
            type,
            status,
            createdAt,
            wallet {
                address,
                bundleHash,
                amount,
                tokenSlug,
                token {
                    name,
                    amount
                },
                tokenUnits {
                    id,
                    name,
                    metas
                }
            },
            fromWallet {
                address,
                bundleHash,
                amount,
                batchId
            },
            toWallet {
                address,
                bundleHash,
                amount,
                batchId
            },
            sourceTokenUnits {
                id,
                name,
                metas
            },
            transferTokenUnits {
                id,
                name,
                metas
            },
            metas {
                key,
                value
            },
            throughMetas {
                key,
                value
            },
            children {
              batchId,
              molecularHash,
              type,
              status,
              createdAt,
              wallet {
                  address,
                  bundleHash,
                  amount,
                  tokenSlug,
                  token {
                      name,
                      amount
                  },
                  tokenUnits {
                      id,
                      name,
                      metas
                  }
              },
              fromWallet {
                  address,
                  bundleHash,
                  amount,
                  batchId
              },
              toWallet {
                  address,
                  bundleHash,
                  amount,
                  batchId
              },
              sourceTokenUnits {
                  id,
                  name,
                  metas
              },
              transferTokenUnits {
                  id,
                  name,
                  metas
              },
              metas {
                  key,
                  value
              },
              throughMetas {
                  key,
                  value
              }
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

            if let Some(ref batch_id) = self.batch_id {
                vars["batchId"] = json!(batch_id);
            }

            Some(vars)
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseMetaBatch::new(json, None).expect("Failed to create ResponseMetaBatch"))
    }
}

/// Convenience methods for common usage patterns
impl QueryBatch {
    /// Query batch by ID (most common pattern)
    pub fn by_id(batch_id: impl Into<String>) -> Self {
        Self::with_batch_id(batch_id)
    }

    /// Query for all batches (no specific batch ID)
    pub fn all() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_batch_creation() {
        let query = QueryBatch::new();
        assert!(query.batch_id().is_none());
    }

    #[test]
    fn test_query_batch_with_batch_id() {
        let query = QueryBatch::with_batch_id("test-batch-id");
        assert_eq!(query.batch_id(), Some("test-batch-id"));
    }

    #[test]
    fn test_set_batch_id() {
        let mut query = QueryBatch::new();
        query.set_batch_id("new-batch-id");
        assert_eq!(query.batch_id(), Some("new-batch-id"));
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_id
        let query = QueryBatch::by_id("batch-123");
        assert_eq!(query.batch_id(), Some("batch-123"));

        // Test all
        let query = QueryBatch::all();
        assert!(query.batch_id().is_none());
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryBatch::with_batch_id("test-batch");
        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["batchId"], json!("test-batch"));
    }

    #[test]
    fn test_compiled_variables_empty() {
        let query = QueryBatch::new();
        let variables = query.compiled_variables(None).unwrap();
        assert!(!variables.as_object().unwrap().contains_key("batchId"));
    }

    #[test]
    fn test_compiled_variables_with_provided() {
        let query = QueryBatch::new();
        let provided_vars = json!({
            "batchId": "provided-batch"
        });
        let variables = query.compiled_variables(Some(provided_vars)).unwrap();
        assert_eq!(variables["batchId"], json!("provided-batch"));
    }

    #[test]
    fn test_query_string() {
        let query = QueryBatch::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("Batch( batchId: $batchId )"));
        assert!(query_string.contains("batchId"));
        assert!(query_string.contains("molecularHash"));
        assert!(query_string.contains("wallet"));
        assert!(query_string.contains("fromWallet"));
        assert!(query_string.contains("toWallet"));
        assert!(query_string.contains("sourceTokenUnits"));
        assert!(query_string.contains("transferTokenUnits"));
        assert!(query_string.contains("metas"));
        assert!(query_string.contains("throughMetas"));
        assert!(query_string.contains("children"));
    }

    #[test]
    fn test_get_fields() {
        let fields = QueryBatch::get_fields();
        assert!(fields.contains("batchId"));
        assert!(fields.contains("molecularHash"));
        assert!(fields.contains("wallet"));
        assert!(fields.contains("tokenUnits"));
    }
}