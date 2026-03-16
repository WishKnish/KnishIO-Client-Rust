//! GraphQL Client Usage Examples
//!
//! This example demonstrates features of the KnishIO Rust SDK's
//! GraphQL client, including queries, mutations, retry policies,
//! and connection configuration.

use knishio_client::{
    GraphQLClient, SocketConfig, RetryPolicy, RetryStrategy,
    RetryCondition, ClientConfig, execute_with_retry,
    create_query_request, create_mutation_request, create_subscription_request,
    KnishIOError,
};
use knishio_client::graphql::RetryConfig;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("KnishIO Rust SDK - GraphQL Client Demo\n");

    // =====================================================
    // 1. Basic Client Creation and Configuration
    // =====================================================

    println!("Creating GraphQL client with custom configuration...");

    let client_config = ClientConfig {
        max_connections: 50,
        connect_timeout: Duration::from_secs(5),
        request_timeout: Duration::from_secs(30),
        keep_alive_timeout: Duration::from_secs(60),
        tcp_keepalive: Some(Duration::from_secs(30)),
        insecure_tls: false,
    };

    let retry_config = RetryConfig {
        max_attempts: 5,
        initial_delay: Duration::from_millis(500),
        ..RetryConfig::default()
    };

    // Note: with_config() returns a GraphQLClient directly (not a builder)
    let mut client = GraphQLClient::with_config(
        "https://api.knish.io/graphql",
        client_config,
        retry_config,
    );

    // Set authentication (in a real app, you'd get this from auth flow)
    client.set_auth_data(
        "your-auth-token-here".to_string(),
        Some("your-public-key".to_string()),
        Some("your-wallet-id".to_string()),
    );

    println!("Client created and configured\n");

    // =====================================================
    // 2. Health Check
    // =====================================================

    println!("Performing health check...");

    match client.health_check().await {
        Ok(is_healthy) => {
            if is_healthy {
                println!("Server is healthy and reachable\n");
            } else {
                println!("Server is not responding correctly\n");
            }
        }
        Err(e) => {
            println!("Health check failed: {}\n", e);
        }
    }

    // =====================================================
    // 3. Simple GraphQL Query
    // =====================================================

    println!("Executing a simple GraphQL query...");

    let balance_query = create_query_request(
        r#"
        query GetBalance($bundleHash: String!, $token: String!) {
            Balance(bundleHash: $bundleHash, token: $token) {
                address
                bundleHash
                amount
                tokenSlug
                createdAt
            }
        }
        "#,
        Some(json!({
            "bundleHash": "your-bundle-hash-here",
            "token": "KNISH"
        })),
    );

    match client.query(balance_query).await {
        Ok(response) => {
            if let Some(data) = &response.data {
                println!("Query successful: {}", serde_json::to_string_pretty(data)?);
            }
            if let Some(errors) = &response.errors {
                println!("Query returned errors: {:?}", errors);
            }
        }
        Err(e) => {
            println!("Query failed: {}", e);
        }
    }
    println!();

    // =====================================================
    // 4. GraphQL Mutation with Custom Headers and Timeout
    // =====================================================

    println!("Executing a GraphQL mutation with custom timeout...");

    let mut create_wallet_mutation = create_mutation_request(
        r#"
        mutation CreateWallet($molecule: MoleculeInput!) {
            ProposeMolecule(molecule: $molecule) {
                molecularHash
                status
                reason
                createdAt
            }
        }
        "#,
        Some(json!({
            "molecule": {
                "molecularHash": "sample-hash",
                "atoms": []
            }
        })),
    );

    // Add custom timeout and headers
    create_wallet_mutation.timeout = Some(Duration::from_secs(45));
    create_wallet_mutation.headers.insert("X-Custom-Header".to_string(), "demo-value".to_string());

    match client.mutate(create_wallet_mutation).await {
        Ok(response) => {
            println!("Mutation successful: {:?}", response.data);
        }
        Err(e) => {
            println!("Mutation failed: {}", e);
        }
    }
    println!();

    // =====================================================
    // 5. WebSocket Subscriptions
    // =====================================================

    println!("Setting up WebSocket subscriptions...");

    // Configure WebSocket
    let socket_config = SocketConfig {
        socket_uri: "wss://api.knish.io/graphql".to_string(),
        app_key: "knishio".to_string(),
        connect_timeout: Some(Duration::from_secs(10)),
        keep_alive_interval: Some(Duration::from_secs(30)),
        max_reconnect_attempts: Some(5),
        reconnect_delay: Some(Duration::from_secs(2)),
    };

    let mut ws_client = GraphQLClient::with_socket(
        "https://api.knish.io/graphql",
        socket_config,
        false, // encrypt
    );

    ws_client.set_auth_data(
        "your-auth-token-here".to_string(),
        Some("your-public-key".to_string()),
        Some("your-wallet-id".to_string()),
    );

    let subscription_request = create_subscription_request(
        r#"
        subscription WalletUpdates($bundleHash: String!) {
            walletStatusUpdated(bundleHash: $bundleHash) {
                address
                amount
                updatedAt
            }
        }
        "#,
        Some(json!({
            "bundleHash": "your-bundle-hash-here"
        })),
        Some("WalletUpdates".to_string()),
    );

    // subscribe() returns Result<()> — the callback receives GraphQLResponse values
    match ws_client.subscribe(subscription_request, |response| {
        println!("Subscription update: {:?}", response);
    }).await {
        Ok(()) => {
            println!("Subscription started");

            // Let it run for a bit
            sleep(Duration::from_secs(5)).await;

            // Unsubscribe from all active subscriptions
            ws_client.unsubscribe_all();
            println!("Unsubscribed from all subscriptions");
        }
        Err(e) => {
            println!("Subscription failed (expected in demo): {}", e);
        }
    }

    println!();

    // =====================================================
    // 6. Custom Retry Policy Demo
    // =====================================================

    println!("Demonstrating custom retry policies...");

    let custom_retry_policy = RetryPolicy::new()
        .with_strategy(RetryStrategy::ExponentialBackoff { multiplier: 1.5 })
        .with_max_attempts(4)
        .with_initial_delay(Duration::from_millis(200))
        .with_max_delay(Duration::from_secs(10))
        .with_jitter(0.2)
        .add_condition(RetryCondition::HttpStatus(503))
        .add_condition(RetryCondition::GraphQLError {
            message_contains: "rate limit".to_string(),
        });

    // Use the retry executor directly
    let mut executor = custom_retry_policy.executor(true);
    let result = executor.execute(|| async {
        // Simulate an operation that might need retries
        Ok::<&str, KnishIOError>("success")
    }).await;

    println!("Retry executor result: {:?}", result);
    println!("Custom retry policy configured\n");

    // =====================================================
    // 7. Connection Statistics
    // =====================================================

    println!("Getting connection statistics...");

    let stats = client.get_stats().await;
    println!("Connection Stats:");
    println!("   - Server URI: {}", stats.server_uri);
    println!("   - Authenticated: {}", stats.is_authenticated);
    println!("   - Encryption: {}", stats.encryption_enabled);
    println!("   - Active Subscriptions: {}", stats.active_subscriptions);
    println!();

    // =====================================================
    // 8. Cleanup
    // =====================================================

    println!("Cleaning up connections...");

    // Unsubscribe from all subscriptions
    client.unsubscribe_all();
    ws_client.unsubscribe_all();

    println!("All connections cleaned up\n");

    // =====================================================
    // 9. Error Handling Demo
    // =====================================================

    println!("Demonstrating error handling...");

    let error_query = create_query_request(
        "query { invalidField }", // This will cause an error
        None,
    );

    match client.query(error_query).await {
        Ok(response) => {
            if let Some(errors) = &response.errors {
                println!("Expected GraphQL errors received: {:?}", errors);
            }
        }
        Err(e) => {
            // KnishIOError supports error categorization
            if e.is_network_error() {
                println!("Network error: {}", e);
            } else if e.is_validation_error() {
                println!("Validation error (expected): {}", e);
            } else {
                println!("Other error: {}", e);
            }
        }
    }

    println!("\nGraphQL Client Demo Complete!");
    println!("\nThis demo showed:");
    println!("   - Client configuration with custom pool and retry settings");
    println!("   - Health checks and connection validation");
    println!("   - GraphQL queries with variables");
    println!("   - GraphQL mutations with custom headers and timeouts");
    println!("   - WebSocket subscriptions");
    println!("   - Custom retry policies with exponential backoff");
    println!("   - Connection statistics and monitoring");
    println!("   - Proper cleanup and resource management");
    println!("   - Error handling and categorization");

    Ok(())
}

/// Example of using the execute_with_retry convenience function
#[allow(dead_code)]
async fn retry_example() -> Result<(), Box<dyn std::error::Error>> {
    let policy = RetryPolicy::network_optimized();

    let result = execute_with_retry(policy, true, || async {
        // Simulate an operation that might fail
        Ok::<&str, KnishIOError>("Success after retries!")
    }).await?;

    println!("Retry result: {}", result);
    Ok(())
}
