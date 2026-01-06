//! Create molecule subscription implementation matching JavaScript SDK
//!
//! Provides real-time notifications for molecule creation events, following
//! the exact patterns from JavaScript CreateMoleculeSubscribe.js

use std::sync::Arc;
use serde_json::Value;
use async_trait::async_trait;
use crate::error::Result;
use crate::graphql::GraphQLClient;
use super::{Subscribe, SubscriptionHandle, SubscriptionManager};

/// Create molecule subscription matching JavaScript CreateMoleculeSubscribe class
pub struct CreateMoleculeSubscribe {
    #[allow(dead_code)]
    graphql_client: Arc<GraphQLClient>,
    subscription_manager: SubscriptionManager,
}

#[async_trait]
impl Subscribe for CreateMoleculeSubscribe {
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
        subscription onCreateMolecule($bundle: String!) {
            CreateMolecule(bundle: $bundle) {
                molecularHash,
                cellSlug,
                counterparty,
                bundleHash,
                status,
                local,
                height,
                depth,
                createdAt,
                receivedAt,
                processedAt,
                broadcastedAt,
                reason,
                reasonPayload,
                payload,
                status,
                atoms {
                    molecularHash,
                    position,
                    isotope,
                    walletAddress,
                    metaType,
                    metaId,
                    value,
                    batchId,
                    createdAt,
                    index,
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
    
    #[tokio::test]
    async fn test_create_molecule_subscribe_creation() {
        let client = Arc::new(GraphQLClient::new("ws://localhost:8080"));
        let subscription = CreateMoleculeSubscribe::new(client);
        
        let query = subscription.get_subscription_query();
        assert!(query.contains("onCreateMolecule"));
        assert!(query.contains("$bundle: String!"));
    }
}