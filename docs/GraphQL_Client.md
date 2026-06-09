# KnishIO Rust SDK - Production-Ready GraphQL Client

This document provides comprehensive documentation for the KnishIO Rust SDK's GraphQL client, which offers full compatibility with the JavaScript SDK while providing enhanced performance and reliability.

## Overview

The GraphQL client provides:

- **HTTP/HTTPS Queries & Mutations**: Full GraphQL operation support with connection pooling
- **WebSocket Subscriptions**: Real-time data streaming with auto-reconnection
- **Advanced Retry Logic**: Configurable retry policies with exponential backoff
- **Connection Pooling**: Efficient resource management for multiple endpoints
- **Authentication Integration**: Seamless token-based authentication
- **Comprehensive Error Handling**: Structured error propagation and recovery
- **Request/Response Formatting**: Compatible with JavaScript SDK format
- **Debug Logging**: Structured logging with tracing support

## Quick Start

### Basic Client Setup

```rust
use knishio_client::{
    GraphQLClient, create_query_request, create_mutation_request
};
use serde_json::json;

// Create a basic client
let mut client = GraphQLClient::new("https://api.knish.io/graphql")
    .with_debug(true);

// Set authentication
client.set_auth_data(
    "your-auth-token".to_string(),
    Some("your-public-key".to_string()),
    Some("your-wallet-id".to_string()),
);

// Execute a query
let query = create_query_request(
    "query { Balance(bundleHash: $hash) { amount } }",
    Some(json!({"hash": "your-bundle-hash"})),
);

let response = client.query(query).await?;
```

### Advanced Configuration

```rust
use knishio_client::{
    GraphQLClient, PoolConfig, RetryPolicy, RetryStrategy
};
use std::time::Duration;

// Configure connection pool
let pool_config = PoolConfig {
    max_connections: 100,
    connect_timeout: Duration::from_secs(10),
    request_timeout: Duration::from_secs(60),
    keep_alive_timeout: Duration::from_secs(90),
    ..PoolConfig::default()
};

// Configure retry policy
let retry_policy = RetryPolicy::network_optimized()
    .with_max_attempts(5)
    .with_initial_delay(Duration::from_millis(500))
    .with_strategy(RetryStrategy::ExponentialBackoff { multiplier: 2.0 });

// Create client with advanced configuration
let client = GraphQLClient::with_config(
    "https://api.knish.io/graphql",
    pool_config,
    retry_policy,
)
.with_debug(true)
.with_timeout(Duration::from_secs(120));
```

## Core Features

### 1. HTTP Operations

#### Queries

```rust
use knishio_client::{create_query_request};
use serde_json::json;

// Create a query request
let query = create_query_request(
    r#"
    query GetWallet($bundleHash: String!, $token: String!) {
        Balance(bundleHash: $bundleHash, token: $token) {
            address
            amount
            tokenSlug
            createdAt
        }
    }
    "#,
    Some(json!({
        "bundleHash": "your-bundle-hash",
        "token": "KNISH"
    })),
);

// Execute the query
let response = client.query(query).await?;

// Process the response
if let Some(data) = response.data {
    println!("Query data: {}", serde_json::to_string_pretty(&data)?);
}

if let Some(errors) = response.errors {
    for error in errors {
        println!("GraphQL error: {}", error.message);
    }
}
```

#### Mutations

```rust
use knishio_client::{create_mutation_request};

let mutation = create_mutation_request(
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
            "molecularHash": "computed-hash",
            "atoms": [/* atom data */]
        }
    })),
);

let response = client.mutate(mutation).await?;
```

#### Custom Headers and Timeouts

```rust
use knishio_client::{GraphQLRequest};
use std::collections::HashMap;
use std::time::Duration;

let mut request = create_query_request(
    "query { __typename }",
    None,
);

// Add custom timeout
request.timeout = Some(Duration::from_secs(30));

// Add custom headers
request.headers.insert("X-Client-Version".to_string(), "1.0.0".to_string());
request.headers.insert("X-Request-ID".to_string(), "unique-id-123".to_string());

let response = client.query(request).await?;
```

### 2. WebSocket Subscriptions

```rust
use knishio_client::{
    GraphQLClient, SocketConfig, create_subscription_request
};
use std::time::Duration;

// Configure WebSocket
let socket_config = SocketConfig {
    socket_uri: "wss://api.knish.io/graphql".to_string(),
    app_key: "knishio".to_string(),
    connect_timeout: Some(Duration::from_secs(10)),
    keep_alive_interval: Some(Duration::from_secs(30)),
    max_reconnect_attempts: Some(5),
    reconnect_delay: Some(Duration::from_secs(2)),
};

// Create client with WebSocket support
let ws_client = GraphQLClient::with_socket(
    "https://api.knish.io/graphql",
    socket_config,
    false, // encrypt
)
.with_debug(true);

// Set authentication
ws_client.set_auth_data(
    "your-auth-token".to_string(),
    Some("your-public-key".to_string()),
    Some("your-wallet-id".to_string()),
);

// Create subscription
let subscription = create_subscription_request(
    r#"
    subscription WalletUpdates($bundleHash: String!) {
        walletStatusUpdated(bundleHash: $bundleHash) {
            address
            amount
            tokenSlug
            updatedAt
        }
    }
    "#,
    Some(json!({
        "bundleHash": "your-bundle-hash"
    })),
    Some("WalletUpdates".to_string()),
);

// Subscribe with callback
let handle = ws_client.subscribe(subscription, |response| {
    match response.data {
        Some(data) => println!("Wallet update: {}", data),
        None => {
            if let Some(errors) = response.errors {
                for error in errors {
                    println!("Subscription error: {}", error.message);
                }
            }
        }
    }
}).await?;

// Later, unsubscribe
ws_client.unsubscribe(&handle.id).await;
```

### 3. Retry Policies

The client supports sophisticated retry logic for handling various failure scenarios.

#### Built-in Policies

```rust
use knishio_client::{RetryPolicy, RetryStrategy, RetryCondition};

// Network-optimized policy
let network_policy = RetryPolicy::network_optimized();

// Rate-limit optimized policy
let rate_limit_policy = RetryPolicy::rate_limit_optimized();

// GraphQL error optimized policy
let graphql_policy = RetryPolicy::graphql_optimized();
```

#### Custom Retry Policy

```rust
let custom_policy = RetryPolicy::new()
    .with_strategy(RetryStrategy::ExponentialBackoff { multiplier: 1.5 })
    .with_max_attempts(5)
    .with_initial_delay(Duration::from_millis(1000))
    .with_max_delay(Duration::from_secs(30))
    .with_jitter(0.1) // Add 10% random jitter
    .with_conditions(vec![
        RetryCondition::NetworkError,
        RetryCondition::ServerError,
        RetryCondition::RateLimit,
        RetryCondition::GraphQLError {
            message_contains: "timeout".to_string(),
        },
    ]);

let client = GraphQLClient::new("https://api.knish.io/graphql")
    .with_retry_config(custom_policy);
```

#### Manual Retry Execution

```rust
use knishio_client::{execute_with_retry};

let policy = RetryPolicy::network_optimized();

let result = execute_with_retry(policy, true, || async {
    // Your operation that might fail
    your_operation().await
}).await?;
```

### 4. Connection Pooling

The client automatically manages HTTP connections for optimal performance.

```rust
use knishio_client::{PoolConfig, global_pool};

// Configure pool settings
let pool_config = PoolConfig {
    max_connections: 200,
    connect_timeout: Duration::from_secs(5),
    request_timeout: Duration::from_secs(30),
    keep_alive_timeout: Duration::from_secs(60),
    cleanup_interval: Duration::from_secs(30),
    user_agent: "MyApp/1.0".to_string(),
};

// Use global pool for efficiency across multiple clients
let pool = global_pool();
let stats = pool.get_stats().await;

println!("Pool stats: {} active connections", stats.total_clients);
```

### 5. Health Checks and Monitoring

```rust
// Health check
let is_healthy = client.health_check().await?;
if !is_healthy {
    println!("Server is not responding correctly");
}

// Connection statistics
let stats = client.get_stats().await;
println!("Connection Stats:");
println!("  Server URI: {}", stats.server_uri);
println!("  Authenticated: {}", stats.is_authenticated);
println!("  Encryption: {}", stats.encryption_enabled);
println!("  Active Subscriptions: {}", stats.active_subscriptions);

// Active subscriptions
let subscription_count = client.subscription_count().await;
let active_subs = client.get_active_subscriptions().await;
println!("Active subscriptions: {} ({:?})", subscription_count, active_subs);
```

### 6. Error Handling

The client provides comprehensive error handling with categorization.

```rust
use knishio_client::KnishIOError;

match client.query(query).await {
    Ok(response) => {
        // Handle successful response
        if let Some(errors) = response.errors {
            // GraphQL errors (query succeeded but had semantic errors)
            for error in errors {
                println!("GraphQL error: {}", error.message);
            }
        }
    }
    Err(error) => {
        // Network or transport errors
        if error.is_network_error() {
            println!("Network error: {}", error);
        } else if error.is_auth_error() {
            println!("Authentication error: {}", error);
        } else if error.is_validation_error() {
            println!("Validation error: {}", error);
        } else {
            println!("Other error: {}", error);
        }
    }
}
```

## Integration with KnishIOClient

The GraphQL client integrates seamlessly with the main KnishIOClient:

```rust
use knishio_client::{KnishIOClient, GraphQLRequest};

// Create main client
let mut client = KnishIOClient::new(
    "https://api.knish.io/graphql",
    None, // cell_slug
    None, // socket config
    None, // custom GraphQL client
    Some(3), // server SDK version
    Some(true), // logging
);

// The client automatically uses the enhanced GraphQL client
let query = create_query_request(
    "query { Balance(bundleHash: $hash) { amount } }",
    Some(json!({"hash": "bundle-hash"})),
);

// Execute through the main client
let response = client.execute_graphql_query(query).await?;

// Health check through main client
let is_healthy = client.health_check().await?;

// Get connection statistics
let stats = client.get_connection_stats().await;
if let Some(stats) = stats {
    println!("Connection stats: {:?}", stats);
}
```

## Performance Optimizations

### 1. Connection Reuse

The client automatically reuses HTTP connections:

```rust
// Multiple requests to the same endpoint reuse connections
for i in 0..100 {
    let query = create_query_request(&format!("query {{ item_{} }}", i), None);
    let response = client.query(query).await?;
    // Connection is reused for optimal performance
}
```

### 2. Request Batching

```rust
// Execute multiple queries concurrently
let queries = vec![
    create_query_request("query { user1: User(id: 1) { name } }", None),
    create_query_request("query { user2: User(id: 2) { name } }", None),
    create_query_request("query { user3: User(id: 3) { name } }", None),
];

let futures: Vec<_> = queries.into_iter()
    .map(|query| client.query(query))
    .collect();

let responses = futures::future::join_all(futures).await;

for result in responses {
    match result {
        Ok(response) => println!("Success: {:?}", response.data),
        Err(error) => println!("Error: {}", error),
    }
}
```

### 3. Resource Cleanup

```rust
// Automatic cleanup when client is dropped
{
    let client = GraphQLClient::new("https://api.knish.io/graphql");
    // ... use client
} // Client and all subscriptions are cleaned up here

// Manual cleanup for long-lived clients
client.unsubscribe_all().await;
```

## Debugging and Logging

Enable detailed logging for troubleshooting:

```rust
// Initialize tracing
tracing_subscriber::init();

// Create client with debug enabled
let client = GraphQLClient::new("https://api.knish.io/graphql")
    .with_debug(true);

// Logs will include:
// - Request/response details
// - Retry attempts and backoff
// - WebSocket connection events
// - Connection pool statistics
// - Authentication events
```

## Configuration Reference

### PoolConfig

```rust
pub struct PoolConfig {
    /// Maximum number of connections per endpoint
    pub max_connections: usize,
    /// Connection timeout for new connections  
    pub connect_timeout: Duration,
    /// Request timeout for individual requests
    pub request_timeout: Duration,
    /// Keep-alive timeout for idle connections
    pub keep_alive_timeout: Duration,
    /// Pool cleanup interval
    pub cleanup_interval: Duration,
    /// User agent string for requests
    pub user_agent: String,
}
```

### RetryPolicy

```rust
pub struct RetryPolicy {
    /// Strategy to use for retries
    pub strategy: RetryStrategy,
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Jitter factor (0.0 to 1.0) to add randomness
    pub jitter_factor: f64,
    /// Conditions that should trigger a retry
    pub retry_conditions: Vec<RetryCondition>,
}
```

### SocketConfig

```rust
pub struct SocketConfig {
    /// WebSocket URI for subscriptions
    pub socket_uri: String,
    /// App key for authentication
    pub app_key: String,
    /// Connection timeout
    pub connect_timeout: Option<Duration>,
    /// Keep-alive interval
    pub keep_alive_interval: Option<Duration>,
    /// Max reconnect attempts
    pub max_reconnect_attempts: Option<u32>,
    /// Reconnect delay
    pub reconnect_delay: Option<Duration>,
}
```

## Best Practices

1. **Reuse Clients**: Create one client instance and reuse it across your application
2. **Enable Debug Logging**: Use `.with_debug(true)` during development
3. **Configure Timeouts**: Set appropriate timeouts based on your use case
4. **Handle Errors Properly**: Use error categorization for appropriate handling
5. **Clean Up Subscriptions**: Always unsubscribe when no longer needed
6. **Monitor Connection Health**: Use health checks for monitoring
7. **Use Retry Policies**: Configure retry policies for your specific error scenarios
8. **Optimize Pool Settings**: Tune connection pool settings for your traffic patterns

## Migration from JavaScript SDK

The Rust GraphQL client maintains API compatibility with the JavaScript SDK:

| JavaScript | Rust |
|------------|------|
| `client.query(request)` | `client.query(request).await` |
| `client.mutate(request)` | `client.mutate(request).await` |
| `client.subscribe(request, callback)` | `client.subscribe(request, callback).await` |
| `client.unsubscribe(operationName)` | `client.unsubscribe(subscription_id).await` |
| `client.setAuthData(token, pubkey, wallet)` | `client.set_auth_data(token, pubkey, wallet)` |
| `client.setUri(uri)` | `client.set_uri(uri)` |
| `client.setEncryption(encrypt)` | `client.set_encryption(encrypt)` |

Response format is identical:

```javascript
// JavaScript
{
  data: { ... },
  errors: [{ message: "...", ... }]
}
```

```rust
// Rust
GraphQLResponse {
    data: Some(...),
    errors: Some(vec![GraphQLError { message: "...", ... }])
}
```

## Troubleshooting

### Common Issues

1. **Connection Timeouts**: Increase `connect_timeout` in `PoolConfig`
2. **Request Timeouts**: Increase `request_timeout` or use custom timeout per request
3. **WebSocket Connection Issues**: Check `socket_uri` and authentication
4. **Retry Not Working**: Verify error matches retry conditions
5. **Memory Usage**: Monitor connection pool and subscription cleanup

### Debug Checklist

1. Enable debug logging: `.with_debug(true)`
2. Check health endpoint: `client.health_check().await`
3. Verify authentication: Check `is_authenticated` in stats
4. Monitor connection pool: Use `get_stats().await`
5. Check active subscriptions: Use `get_active_subscriptions().await`

For more examples, see the [examples directory](../examples/) in the repository.
