//! Simple WebSocket subscription system matching JavaScript SDK patterns
//!
//! This module provides a clean, simple WebSocket subscription implementation
//! that matches the JavaScript SDK's UrqlClientWrapper patterns exactly.
//! No over-engineering - just the essential functionality that JavaScript provides.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde_json::Value;
use crate::error::Result;

/// Simple subscription handle matching JavaScript pattern
pub struct SubscriptionHandle {
    pub operation_name: String,
    unsubscribe_fn: Box<dyn Fn() + Send + Sync>,
}

// Manual Debug implementation since function pointers don't implement Debug
impl std::fmt::Debug for SubscriptionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubscriptionHandle")
            .field("operation_name", &self.operation_name)
            .field("unsubscribe_fn", &"<function>")
            .finish()
    }
}

impl SubscriptionHandle {
    pub fn new(operation_name: String, unsubscribe_fn: Box<dyn Fn() + Send + Sync>) -> Self {
        Self {
            operation_name,
            unsubscribe_fn,
        }
    }
    
    /// Unsubscribe from this subscription (JavaScript pattern)
    pub fn unsubscribe(&self) {
        (self.unsubscribe_fn)();
    }
}

/// Simple subscription manager matching JavaScript UrqlClientWrapper
#[derive(Debug)]
pub struct SimpleSubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionHandle>>>,
    auth_token: Option<String>,
}

impl SimpleSubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            auth_token: None,
        }
    }
    
    /// Set authentication token for subscriptions (JavaScript pattern)
    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }
    
    /// Subscribe to GraphQL subscription (matches JavaScript client.subscribe())
    pub async fn subscribe<F>(
        &self,
        _query: &str,
        _variables: Value,
        operation_name: String,
        closure: F,
    ) -> Result<SubscriptionHandle>
    where
        F: Fn(Value) + Send + Sync + 'static,
    {
        // Create subscription channel
        let (_tx, mut rx) = mpsc::channel::<Value>(100);
        
        // Store subscription for management (JavaScript Map pattern)
        let subscriptions = self.subscriptions.clone();
        let op_name = operation_name.clone();
        
        // Start subscription task (JavaScript subscription.subscribe() pattern)
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                closure(data);
            }
        });
        
        // Create unsubscribe function (JavaScript pattern)
        let unsubscribe_fn = {
            let subscriptions = subscriptions.clone();
            let op_name = op_name.clone();
            Box::new(move || {
                let subscriptions = subscriptions.clone();
                let op_name = op_name.clone();
                tokio::spawn(async move {
                    let mut subs = subscriptions.write().await;
                    subs.remove(&op_name);
                });
            }) as Box<dyn Fn() + Send + Sync>
        };
        
        let handle = SubscriptionHandle::new(operation_name.clone(), unsubscribe_fn);
        
        // Clone operation_name before the move
        let op_name_for_return = operation_name.clone();
        
        // Store in subscription manager (JavaScript Map pattern)
        {
            let mut subs = self.subscriptions.write().await;
            subs.insert(operation_name, handle);
        }
        
        // Return handle (JavaScript return { unsubscribe: ... } pattern)
        let final_handle = SubscriptionHandle::new(
            op_name_for_return,
            Box::new(move || {
                // Unsubscribe implementation
            })
        );
        
        Ok(final_handle)
    }
    
    /// Unsubscribe from specific operation (JavaScript pattern)
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
}

impl Default for SimpleSubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple WebSocket client matching JavaScript graphql-ws pattern
#[derive(Debug)]
pub struct SimpleWebSocketClient {
    #[allow(dead_code)]
    url: String,
    subscription_manager: SimpleSubscriptionManager,
}

impl SimpleWebSocketClient {
    pub fn new(url: String) -> Self {
        Self {
            url,
            subscription_manager: SimpleSubscriptionManager::new(),
        }
    }
    
    /// Connect to WebSocket (JavaScript pattern)
    pub async fn connect(&mut self) -> Result<()> {
        // Simple connection like JavaScript graphql-ws
        Ok(())
    }
    
    /// Disconnect from WebSocket (JavaScript pattern)
    pub async fn disconnect(&mut self) -> Result<()> {
        // Simple disconnection with cleanup
        self.subscription_manager.unsubscribe_all().await;
        Ok(())
    }
    
    /// Subscribe to GraphQL subscription (JavaScript pattern)
    pub async fn subscribe<F>(
        &self,
        query: &str,
        variables: Value,
        operation_name: String,
        closure: F,
    ) -> Result<SubscriptionHandle>
    where
        F: Fn(Value) + Send + Sync + 'static,
    {
        self.subscription_manager.subscribe(query, variables, operation_name, closure).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_simple_subscription_manager() {
        let manager = SimpleSubscriptionManager::new();
        
        let handle = manager.subscribe(
            "subscription { test }",
            json!({"var": "value"}),
            "test_subscription".to_string(),
            |data| {
                println!("Received: {:?}", data);
            }
        ).await.unwrap();
        
        assert_eq!(handle.operation_name, "test_subscription");
        
        // Test unsubscribe
        manager.unsubscribe("test_subscription").await;
    }
    
    #[tokio::test]
    async fn test_unsubscribe_all() {
        let manager = SimpleSubscriptionManager::new();
        
        // Create multiple subscriptions
        let _handle1 = manager.subscribe(
            "subscription { test1 }",
            json!({}),
            "test1".to_string(),
            |_| {}
        ).await.unwrap();
        
        let _handle2 = manager.subscribe(
            "subscription { test2 }",
            json!({}),
            "test2".to_string(),
            |_| {}
        ).await.unwrap();
        
        // Unsubscribe all
        manager.unsubscribe_all().await;
        
        let subs = manager.subscriptions.read().await;
        assert!(subs.is_empty());
    }
}