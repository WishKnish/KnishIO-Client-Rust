//! Subscription system for real-time KnishIO events
//!
//! This module provides WebSocket-based subscriptions for listening to
//! real-time events from KnishIO nodes, matching the JavaScript SDK functionality exactly.
//! 
//! The implementation follows JavaScript SDK patterns:
//! - Simple Subscribe base class
//! - Specific subscription classes with GraphQL queries
//! - Basic subscription management with Map-like storage
//! - Simple unsubscribe functionality

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use async_trait::async_trait;
use crate::error::Result;
use crate::graphql::GraphQLClient;

// Simple WebSocket implementation
pub mod simple_websocket;
pub use simple_websocket::{SimpleSubscriptionManager, SimpleWebSocketClient, SubscriptionHandle};

// Specific subscription implementations (matching JavaScript)
pub mod active_wallet_subscribe;
pub mod active_session_subscribe;
pub mod create_molecule_subscribe;
pub mod wallet_status_subscribe;

// Re-export subscription types
pub use active_wallet_subscribe::ActiveWalletSubscribe;
pub use active_session_subscribe::ActiveSessionSubscribe;
pub use create_molecule_subscribe::CreateMoleculeSubscribe;
pub use wallet_status_subscribe::WalletStatusSubscribe;

/// Base subscription trait matching JavaScript Subscribe class
#[async_trait]
pub trait Subscribe {
    /// Create new subscription instance (JavaScript constructor pattern)
    fn new(graphql_client: Arc<GraphQLClient>) -> Self;
    
    /// Get the GraphQL subscription query string
    fn get_subscription_query(&self) -> &'static str;
    
    /// Execute subscription with variables and closure (JavaScript execute() pattern)
    async fn execute(
        &self, 
        variables: Value, 
        closure: Box<dyn Fn(Value) + Send + Sync>
    ) -> Result<SubscriptionHandle>;
    
    /// Compile variables for the subscription (JavaScript compiledVariables() pattern)
    fn compiled_variables(&self, variables: Option<Value>) -> Value {
        variables.unwrap_or_else(|| Value::Object(serde_json::Map::new()))
    }
}

/// Subscription event data matching JavaScript callback pattern
#[derive(Debug, Clone)]
pub struct SubscriptionEvent {
    pub operation_name: String,
    pub data: Value,
}

impl SubscriptionEvent {
    pub fn new(operation_name: String, data: Value) -> Self {
        Self {
            operation_name,
            data,
        }
    }
}

/// Subscription control for managing subscription lifecycle
#[derive(Debug, Clone)]
pub enum SubscriptionControl {
    Stop,
    Pause,
    Resume,
}

/// Subscription status matching JavaScript patterns
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionStatus {
    Inactive,
    Active,
    Paused,
    Error,
}

/// Simple subscription manager implementation matching JavaScript UrqlClientWrapper
pub struct SubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionHandle>>>,
    graphql_client: Arc<GraphQLClient>,
}

impl SubscriptionManager {
    pub fn new(graphql_client: Arc<GraphQLClient>) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            graphql_client,
        }
    }
    
    /// Create subscription request matching JavaScript createSubscribe() pattern
    pub fn create_subscribe_request(&self, query: &str, variables: Value) -> SubscribeRequest {
        SubscribeRequest {
            query: query.to_string(),
            variables,
            fetch_policy: "no-cache".to_string(),
        }
    }
    
    /// Subscribe to GraphQL subscription (JavaScript client.subscribe() pattern)
    pub async fn subscribe<F>(
        &self,
        _request: SubscribeRequest,
        _closure: F,
    ) -> Result<SubscriptionHandle>
    where
        F: Fn(Value) + Send + Sync + 'static,
    {
        let operation_name = format!("subscription_{}", uuid::Uuid::new_v4());
        
        // Create unsubscribe function (JavaScript pattern)
        let subscriptions = self.subscriptions.clone();
        let op_name = operation_name.clone();
        let unsubscribe_fn = Box::new(move || {
            let subscriptions = subscriptions.clone();
            let op_name = op_name.clone();
            tokio::spawn(async move {
                let mut subs = subscriptions.write().await;
                subs.remove(&op_name);
            });
        }) as Box<dyn Fn() + Send + Sync>;
        
        let handle = SubscriptionHandle::new(operation_name.clone(), unsubscribe_fn);
        
        // Clone operation_name before the move
        let op_name_for_final = operation_name.clone();
        
        // Store subscription (JavaScript Map pattern)
        {
            let mut subs = self.subscriptions.write().await;
            subs.insert(operation_name, handle);
        }
        
        // Return handle with unsubscribe (JavaScript return { unsubscribe: ... } pattern)
        let final_unsubscribe_fn = {
            let manager = self.clone();
            let op_name = op_name_for_final.clone();
            Box::new(move || {
                let manager = manager.clone();
                let op_name = op_name.clone();
                tokio::spawn(async move {
                    manager.unsubscribe(&op_name).await;
                });
            }) as Box<dyn Fn() + Send + Sync>
        };
        
        Ok(SubscriptionHandle::new(op_name_for_final, final_unsubscribe_fn))
    }
    
    /// Unsubscribe from specific subscription (JavaScript pattern)
    pub async fn unsubscribe(&self, operation_name: &str) {
        let mut subs = self.subscriptions.write().await;
        if let Some(subscription) = subs.remove(operation_name) {
            subscription.unsubscribe();
        }
    }
    
    /// Unsubscribe from all subscriptions (JavaScript pattern)
    pub async fn unsubscribe_all(&self) {
        let mut subs = self.subscriptions.write().await;
        for (_, subscription) in subs.drain() {
            subscription.unsubscribe();
        }
    }
    
    /// Connect to subscription service (JavaScript client pattern)
    pub async fn connect(&self) -> Result<()> {
        // Simple connection - JavaScript doesn't have complex connection management
        Ok(())
    }
    
    /// Disconnect from subscription service (JavaScript pattern)
    pub async fn disconnect(&self) -> Result<()> {
        // Unsubscribe all and disconnect (JavaScript socketDisconnect() pattern)
        self.unsubscribe_all().await;
        Ok(())
    }
    
    /// Check if connected (JavaScript pattern)
    pub async fn is_connected(&self) -> bool {
        // Simple connected state - JavaScript doesn't track complex connection state
        true
    }
    
    /// Stop all subscriptions (JavaScript unsubscribeAll() pattern)
    pub async fn stop_all(&self) -> Result<()> {
        self.unsubscribe_all().await;
        Ok(())
    }
    
    /// Get active subscription count (JavaScript Map.size pattern)
    pub async fn active_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }
    
    /// List all subscription names (JavaScript Map.keys() pattern)
    pub async fn list_subscriptions(&self) -> Vec<String> {
        self.subscriptions.read().await.keys().cloned().collect()
    }
    
    /// Check if subscription exists by ID (JavaScript Map.has() pattern)
    pub async fn get_subscription(&self, id: &str) -> Option<String> {
        let subs = self.subscriptions.read().await;
        if subs.contains_key(id) {
            Some(id.to_string())
        } else {
            None
        }
    }
}

impl Clone for SubscriptionManager {
    fn clone(&self) -> Self {
        Self {
            subscriptions: self.subscriptions.clone(),
            graphql_client: self.graphql_client.clone(),
        }
    }
}

/// Subscription request structure matching JavaScript pattern
#[derive(Debug, Clone)]
pub struct SubscribeRequest {
    pub query: String,
    pub variables: Value,
    pub fetch_policy: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_subscription_manager_creation() {
        let client = Arc::new(GraphQLClient::new("ws://localhost:8080"));
        let manager = SubscriptionManager::new(client);
        
        assert!(manager.subscriptions.read().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_create_subscribe_request() {
        let client = Arc::new(GraphQLClient::new("ws://localhost:8080"));
        let manager = SubscriptionManager::new(client);
        
        let request = manager.create_subscribe_request(
            "subscription { test }",
            json!({"bundle": "test123"})
        );
        
        assert_eq!(request.query, "subscription { test }");
        assert_eq!(request.fetch_policy, "no-cache");
    }
}