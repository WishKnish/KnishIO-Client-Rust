//! Comprehensive GraphQL Client Usage Examples
//!
//! This example demonstrates all the features of the KnishIO Rust SDK's
//! production-ready GraphQL client, including queries, mutations,
//! subscriptions, retry policies, and connection pooling.

use knishio_client::{
    GraphQLClient, GraphQLRequest, SocketConfig, RetryPolicy, RetryStrategy,
    RetryCondition, ClientConfig, create_query_request, create_mutation_request,
    create_subscription_request, KnishIOError,
};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better debugging
    tracing_subscriber::init();
    
    println!("🚀 KnishIO Rust SDK - GraphQL Client Demo\n");
    
    // =====================================================
    // 1. Basic Client Creation and Configuration
    // =====================================================
    
    println!("📡 Creating GraphQL client with custom configuration...");
    
    let pool_config = ClientConfig {
        max_connections: 50,
        connect_timeout: Duration::from_secs(5),
        request_timeout: Duration::from_secs(30),
        keep_alive_timeout: Duration::from_secs(60),
        cleanup_interval: Duration::from_secs(30),
        user_agent: "KnishIO-Demo/1.0".to_string(),
        ..ClientConfig::default()
    };
    
    let retry_policy = RetryPolicy::network_optimized()
        .with_max_attempts(5)
        .with_initial_delay(Duration::from_millis(500));
    
    let mut client = GraphQLClient::with_config(
        "https://api.knish.io/graphql",
        pool_config,
        retry_policy,
    )
    .with_debug(true)
    .with_timeout(Duration::from_secs(60));
    
    // Set authentication (in a real app, you'd get this from auth flow)
    client.set_auth_data(
        "your-auth-token-here".to_string(),
        Some("your-public-key".to_string()),
        Some("your-wallet-id".to_string()),
    );
    
    println!("✅ Client created and configured\n");
    
    // =====================================================
    // 2. Health Check
    // =====================================================
    
    println!("🔍 Performing health check...");
    
    match client.health_check().await {
        Ok(is_healthy) => {
            if is_healthy {
                println!("✅ Server is healthy and reachable\n");
            } else {
                println!("⚠️  Server is not responding correctly\n");
            }
        }
        Err(e) => {
            println!("❌ Health check failed: {}\n", e);
        }
    }
    
    // =====================================================
    // 3. Simple GraphQL Query
    // =====================================================
    
    println!("📊 Executing a simple GraphQL query...");
    
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
                println!("✅ Query successful: {}", serde_json::to_string_pretty(data)?);
            }
            if let Some(errors) = &response.errors {
                println!("⚠️  Query returned errors: {:?}", errors);
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }
    println!();
    
    // =====================================================
    // 4. GraphQL Mutation with Custom Headers and Timeout
    // =====================================================
    
    println!("✏️  Executing a GraphQL mutation with custom timeout...");
    
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
            println!("✅ Mutation successful: {:?}", response.data);
        }
        Err(e) => {
            println!("❌ Mutation failed: {}", e);
        }
    }
    println!();
    
    // =====================================================
    // 5. WebSocket Subscriptions (if available)
    // =====================================================
    
    println!("🔄 Setting up WebSocket subscriptions...");
    
    // Configure WebSocket
    let socket_config = SocketConfig {
        socket_uri: "wss://api.knish.io/graphql".to_string(),
        app_key: "knishio".to_string(),
        connect_timeout: Some(Duration::from_secs(10)),
        keep_alive_interval: Some(Duration::from_secs(30)),
        max_reconnect_attempts: Some(5),
        reconnect_delay: Some(Duration::from_secs(2)),
    };
    
    let ws_client = GraphQLClient::with_socket(
        "https://api.knish.io/graphql",
        socket_config,
        false, // encrypt
    )
    .with_debug(true);
    
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
    
    // Note: In a real application, you'd handle the subscription differently
    // This is just a demonstration of the API
    let subscription_result = ws_client.subscribe(subscription_request, |response| {
        println!("📨 Subscription update: {:?}", response);
    }).await;
    
    match subscription_result {
        Ok(handle) => {
            println!("✅ Subscription started with ID: {}", handle.id);
            
            // Let it run for a bit
            sleep(Duration::from_secs(5)).await;
            
            // Unsubscribe
            ws_client.unsubscribe(&handle.id).await;
            println!("✅ Unsubscribed from {}", handle.id);
        }
        Err(e) => {
            println!("❌ Subscription failed: {}", e);
        }
    }
    
    println!();
    
    // =====================================================
    // 6. Custom Retry Policy Demo
    // =====================================================
    
    println!("🔄 Demonstrating custom retry policies...");
    
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
    
    let retry_client = GraphQLClient::new("https://api.knish.io/graphql")
        .with_retry_config(custom_retry_policy)
        .with_debug(true);
    
    // This would demonstrate retries, but we'll simulate it
    println!("✅ Custom retry policy configured\n");
    
    // =====================================================
    // 7. Connection Statistics
    // =====================================================
    
    println!("📈 Getting connection statistics...");
    
    let stats = client.get_stats().await;
    println!("📊 Connection Stats:");
    println!("   - Server URI: {}", stats.server_uri);
    println!("   - Authenticated: {}", stats.is_authenticated);
    println!("   - Encryption: {}", stats.encryption_enabled);
    println!("   - Active Subscriptions: {}", stats.active_subscriptions);
    println!();
    
    // =====================================================
    // 8. Cleanup
    // =====================================================
    
    println!("🧹 Cleaning up connections...");
    
    // Unsubscribe from all subscriptions
    client.unsubscribe_all().await;
    ws_client.unsubscribe_all().await;
    
    println!("✅ All connections cleaned up\n");
    
    // =====================================================
    // 9. Advanced Error Handling Demo
    // =====================================================
    
    println!("🚨 Demonstrating advanced error handling...");
    
    let error_query = create_query_request(
        "query { invalidField }", // This will cause an error
        None,
    );
    
    match client.query(error_query).await {
        Ok(response) => {
            if let Some(errors) = &response.errors {
                println!("⚠️  Expected GraphQL errors received: {:?}", errors);
            }
        }
        Err(e) => {
            // Demonstrate error categorization
            if e.is_network_error() {
                println!("🌐 Network error: {}", e);
            } else if e.is_validation_error() {
                println!("✅ Validation error (expected): {}", e);
            } else {
                println!("❓ Other error: {}", e);
            }
        }
    }
    
    println!("\n🎉 GraphQL Client Demo Complete!");
    println!("\n📚 This demo showed:");
    println!("   ✓ Client configuration with custom pool and retry settings");
    println!("   ✓ Health checks and connection validation");
    println!("   ✓ GraphQL queries with variables");
    println!("   ✓ GraphQL mutations with custom headers and timeouts");
    println!("   ✓ WebSocket subscriptions with auto-reconnection");
    println!("   ✓ Custom retry policies with exponential backoff");
    println!("   ✓ Connection statistics and monitoring");
    println!("   ✓ Proper cleanup and resource management");
    println!("   ✓ Advanced error handling and categorization");
    
    Ok(())
}

/// Helper function to demonstrate retry policy in action
async fn simulate_retry_scenario() -> Result<String, KnishIOError> {
    // This would simulate a flaky operation that sometimes fails
    static mut ATTEMPT_COUNT: u32 = 0;
    
    unsafe {
        ATTEMPT_COUNT += 1;
        
        if ATTEMPT_COUNT < 3 {
            // Simulate network failure
            return Err(KnishIOError::custom("HTTP error: 503 Service Unavailable"));
        }
        
        Ok("Success after retries!".to_string())
    }
}

/// Example of using the execute_with_retry helper function
async fn retry_example() -> Result<(), Box<dyn std::error::Error>> {
    use knishio_client::execute_with_retry;
    
    let policy = RetryPolicy::network_optimized();
    
    let result = execute_with_retry(policy, true, || async {
        simulate_retry_scenario().await
    }).await?;
    
    println!("Retry result: {}", result);
    Ok(())
}