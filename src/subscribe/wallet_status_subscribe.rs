//! Wallet status subscription implementation matching JavaScript SDK
//!
//! Provides real-time notifications for wallet status changes, following
//! the exact patterns from JavaScript WalletStatusSubscribe.js

use std::sync::Arc;
use serde_json::Value;
use async_trait::async_trait;
use crate::error::Result;
use crate::graphql::GraphQLClient;
use super::{Subscribe, SubscriptionHandle, SubscriptionManager};

/// Wallet status subscription matching JavaScript WalletStatusSubscribe class
pub struct WalletStatusSubscribe {
    #[allow(dead_code)]
    graphql_client: Arc<GraphQLClient>,
    subscription_manager: SubscriptionManager,
}

#[async_trait]
impl Subscribe for WalletStatusSubscribe {
    /// Create new instance (JavaScript constructor pattern)
    fn new(graphql_client: Arc<GraphQLClient>) -> Self {
        let subscription_manager = SubscriptionManager::new(graphql_client.clone());
        
        Self {
            graphql_client,
            subscription_manager,
        }
    }
    
    /// Get GraphQL subscription query (matches JavaScript $__subscribe)
    fn get_subscription_query(&self) -> &'static str {
        r#"
        subscription onWalletStatus($bundle: String!, $token: String!) {
            WalletStatus(bundle: $bundle, token: $token) {
                address,
                position,
                amount,
                characters,
                batchId,
                createdAt,
                token {
                    slug,
                    name,
                    fungibility,
                    supply,
                    decimals,
                    amount,
                    icon,
                    createdAt
                },
                metas {
                    molecularHash,
                    position,
                    metaType,
                    metaId,
                    key,
                    value,
                    createdAt,
                }
            }
        }
        "#
    }
    
    /// Execute subscription (JavaScript execute() pattern)
    async fn execute(
        &self, 
        variables: Value, 
        closure: Box<dyn Fn(Value) + Send + Sync>
    ) -> Result<SubscriptionHandle> {
        // Create subscription request (JavaScript createSubscribe() pattern)
        let request = self.subscription_manager.create_subscribe_request(
            self.get_subscription_query(),
            self.compiled_variables(Some(variables))
        );
        
        // Execute subscription (JavaScript client.subscribe() pattern)
        self.subscription_manager.subscribe(request, closure).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_wallet_status_subscribe_creation() {
        let client = Arc::new(GraphQLClient::new("ws://localhost:8080"));
        let subscription = WalletStatusSubscribe::new(client);
        
        let query = subscription.get_subscription_query();
        assert!(query.contains("onWalletStatus"));
        assert!(query.contains("$bundle: String!"));
        assert!(query.contains("$token: String!"));
    }
}