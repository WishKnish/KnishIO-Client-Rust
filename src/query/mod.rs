//! GraphQL query implementations
//!
//! This module contains query classes for retrieving data from KnishIO nodes,
//! equivalent to the JavaScript query classes.

use crate::error::{KnishIOError, Result};
use crate::graphql::{GraphQLClient, GraphQLRequest, create_query_request};
use crate::response::Response;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Base Query trait for all query implementations
#[async_trait::async_trait]
pub trait Query: Send + Sync {
    /// Get the GraphQL query string
    fn get_query(&self) -> &str;
    
    /// Get compiled variables for the query
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value>;
    
    /// Create a response from the JSON data
    fn create_response(&self, json: Value) -> Box<dyn Response>;
    
    /// Create query context for authentication
    fn create_query_context(&self) -> HashMap<String, Value> {
        HashMap::new()
    }
    
    /// Execute the query
    async fn execute(
        &self,
        client: &GraphQLClient,
        variables: Option<Value>,
        _context: Option<HashMap<String, Value>>,
    ) -> Result<Box<dyn Response>> {
        let compiled_vars = self.compiled_variables(variables);
        let request = create_query_request(self.get_query(), compiled_vars);

        let response = client.query(request).await?;

        // Convert GraphQLResponse to our Response type
        let json_data = response.data.unwrap_or_else(|| json!({}));
        Ok(self.create_response(json_data))
    }
}

/// Base Query implementation (equivalent to Query.js)
pub struct BaseQuery {
    query_string: String,
    variables: Option<Value>,
    response: Option<Box<dyn Response>>,
    request: Option<GraphQLRequest>,
    compiled_vars: Option<Value>,
}

impl BaseQuery {
    /// Create a new BaseQuery with the given GraphQL query string
    pub fn new(query_string: impl Into<String>) -> Self {
        BaseQuery {
            query_string: query_string.into(),
            variables: None,
            response: None,
            request: None,
            compiled_vars: None,
        }
    }
    
    /// Set variables for the query
    pub fn with_variables(mut self, variables: Value) -> Self {
        self.variables = Some(variables);
        self
    }
    
    /// Get the stored response (equivalent to response() in JS)
    pub fn response(&self) -> Option<&Box<dyn Response>> {
        self.response.as_ref()
    }
    
    /// Get the compiled variables (equivalent to variables() in JS)
    pub fn variables(&self) -> Option<&Value> {
        self.compiled_vars.as_ref()
    }
    
    /// Get the stored request
    pub fn request(&self) -> Option<&GraphQLRequest> {
        self.request.as_ref()
    }
    
    /// Create a query request (equivalent to createQuery in JS)
    pub fn create_query(&mut self, variables: Option<Value>) -> Result<GraphQLRequest> {
        // Compile variables
        self.compiled_vars = self.compiled_variables(variables);
        
        // Validate query string is not empty
        if self.query_string.is_empty() {
            return Err(KnishIOError::Code("Query string was not initialized!".to_string()));
        }
        
        // Create the request
        let request = create_query_request(&self.query_string, self.compiled_vars.clone());
        self.request = Some(request.clone());
        
        Ok(request)
    }
    
    /// Create a raw response (equivalent to createResponseRaw in JS)
    pub fn create_response_raw(&self, response: Value) -> Box<dyn Response> {
        self.create_response(response)
    }
}

#[async_trait::async_trait]
impl Query for BaseQuery {
    fn get_query(&self) -> &str {
        &self.query_string
    }
    
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        variables.or_else(|| self.variables.clone())
    }
    
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        match crate::response::BaseResponse::new(json) {
            Ok(resp) => Box::new(resp),
            Err(e) => {
                eprintln!("BaseResponse construction failed: {}", e);
                Box::new(crate::response::BaseResponse::empty())
            }
        }
    }
    
    /// Execute the query with enhanced functionality matching JS
    async fn execute(
        &self,
        client: &GraphQLClient,
        variables: Option<Value>,
        context: Option<HashMap<String, Value>>,
    ) -> Result<Box<dyn Response>> {
        // Create a mutable clone to store state
        let mut query = BaseQuery {
            query_string: self.query_string.clone(),
            variables: self.variables.clone(),
            response: None,
            request: None,
            compiled_vars: None,
        };
        
        // Create the query request
        let request = query.create_query(variables)?;
        
        // Merge context
        let mut merged_context = context.unwrap_or_default();
        merged_context.extend(self.create_query_context());
        
        // Execute the query
        match client.query(request).await {
            Ok(response) => {
                // Convert GraphQLResponse to our Response type
                let json_data = response.data.unwrap_or_else(|| json!({}));
                let response_obj = query.create_response_raw(json_data);
                
                // Note: In Rust, we can't mutate self in an async trait method
                // The response is returned directly instead of being stored
                Ok(response_obj)
            }
            Err(e) => {
                // Handle cancellation specifically
                if e.to_string().contains("cancelled") || e.to_string().contains("abort") {
                    let cancelled_response = json!({
                        "data": null,
                        "errors": [{ "message": "Query was cancelled" }]
                    });
                    Ok(query.create_response_raw(cancelled_response))
                } else {
                    Err(e)
                }
            }
        }
    }
}

// Specific query type implementations
pub mod active_session;
pub mod atom;
pub mod balance;
pub mod batch;
pub mod batch_history;
pub mod continu_id;
pub mod meta_type;
pub mod meta_type_via_atom;
pub mod policy;
pub mod token;
pub mod user_activity;
pub mod wallet_bundle;
pub mod wallet_list;

// Re-export query classes
pub use active_session::QueryActiveSession;
pub use atom::{QueryAtom, QueryAtomParams};
pub use balance::QueryBalance;
pub use batch::QueryBatch;
pub use batch_history::QueryBatchHistory;
pub use continu_id::QueryContinuId;
pub use meta_type::{QueryMetaType, MetaTypeValue};
pub use meta_type_via_atom::{QueryMetaTypeViaAtom, QueryMetaTypeViaAtomParams};
pub use policy::QueryPolicy;
pub use token::QueryToken;
pub use user_activity::QueryUserActivity;
pub use wallet_bundle::QueryWalletBundle;
pub use wallet_list::QueryWalletList;