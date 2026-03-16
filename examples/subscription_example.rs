//! Example demonstrating the complete WebSocket subscription system
//!
//! This example shows how to use all 4 subscription types in the Rust SDK
//! to match the functionality available in the JavaScript SDK.

use knishio_client::{
    KnishIOClient,
    GraphQLClient,
};
use knishio_client::subscribe::{
    Subscribe, SubscriptionManager, SubscriptionEvent,
    CreateMoleculeSubscribe, ActiveWalletSubscribe, ActiveSessionSubscribe, WalletStatusSubscribe,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("KnishIO Rust SDK Subscription System Demo");
    println!("==========================================");

    // Initialize the KnishIO client
    let client = KnishIOClient::new(
        "ws://localhost:8080".to_string(),  // WebSocket endpoint
        Some("default".to_string()),        // Cell slug
        None,                               // Socket config (auto-generated)
        None,                               // GraphQL client (auto-generated)
        Some(3),                            // Server SDK version
        Some(true),                         // Enable logging
    );

    // Set a secret for the client session
    // client.set_secret("example-secret-for-testing-12345");

    println!("\nTesting CreateMolecule Subscription");
    println!("-----------------------------------");

    // 1. CreateMolecule Subscription - Listen for new transactions
    let create_molecule_handle = client.subscribe_create_molecule(
        Some("test-bundle-hash".to_string()),
        |event: SubscriptionEvent| {
            println!("[{}] New Molecule Created: {}", event.operation_name, event.data);
            if let Some(hash) = event.data.get("molecularHash") {
                println!("   Hash: {}", hash);
            }
            if let Some(status) = event.data.get("status") {
                println!("   Status: {}", status);
            }
        }
    ).await?;

    println!("CreateMolecule subscription active: {}", create_molecule_handle.operation_name);

    println!("\nTesting ActiveWallet Subscription");
    println!("---------------------------------");

    // 2. ActiveWallet Subscription - Listen for wallet updates
    let active_wallet_handle = client.subscribe_active_wallet(
        Some("test-bundle-hash".to_string()),
        |event: SubscriptionEvent| {
            println!("[{}] Wallet Update: {}", event.operation_name, event.data);
            if let Some(address) = event.data.get("address") {
                println!("   Address: {}", address);
            }
            if let Some(balance) = event.data.get("amount") {
                println!("   Balance: {}", balance);
            }
        }
    ).await?;

    println!("ActiveWallet subscription active: {}", active_wallet_handle.operation_name);

    println!("\nTesting WalletStatus Subscription");
    println!("---------------------------------");

    // 3. WalletStatus Subscription - Monitor wallet status changes
    let wallet_status_handle = client.subscribe_wallet_status(
        Some("test-bundle-hash".to_string()),
        "TEST".to_string(),  // Token type
        |event: SubscriptionEvent| {
            println!("[{}] Wallet Status Change: {}", event.operation_name, event.data);
            if let Some(balance) = event.data.get("balance") {
                println!("   New Balance: {}", balance);
            }
        }
    ).await?;

    println!("WalletStatus subscription active: {}", wallet_status_handle.operation_name);

    println!("\nTesting ActiveSession Subscription");
    println!("----------------------------------");

    // 4. ActiveSession Subscription - Monitor user activity
    let active_session_handle = client.subscribe_active_session(
        "user".to_string(),      // Meta type
        "user123".to_string(),   // Meta ID
        |event: SubscriptionEvent| {
            println!("[{}] Session Activity: {}", event.operation_name, event.data);
            if let Some(bundle) = event.data.get("bundleHash") {
                println!("   User: {}", bundle);
            }
        }
    ).await?;

    println!("ActiveSession subscription active: {}", active_session_handle.operation_name);

    println!("\nSubscription Manager Status");
    println!("===========================");

    // Get subscription manager status
    let manager = client.get_subscription_manager()?;
    println!("Active subscriptions: {}", manager.active_count().await);
    println!("WebSocket connected: {}", manager.is_connected().await);

    // List all subscription IDs
    let subscription_ids = manager.list_subscriptions().await;
    println!("Subscription IDs:");
    for id in &subscription_ids {
        println!("   - {}", id);
    }

    println!("\nRunning subscriptions for 30 seconds...");
    println!("   (In a real application, subscriptions would receive actual events)");

    // Let subscriptions run for a bit
    for i in 1..=6 {
        sleep(Duration::from_secs(5)).await;
        println!("{}s elapsed... subscriptions still active", i * 5);
        println!("   Active count: {}", manager.active_count().await);
    }

    println!("\nStopping All Subscriptions");
    println!("==========================");

    // Unsubscribe from individual subscriptions
    println!("Stopping CreateMolecule subscription...");
    create_molecule_handle.unsubscribe();

    println!("Stopping ActiveWallet subscription...");
    active_wallet_handle.unsubscribe();

    println!("Stopping WalletStatus subscription...");
    wallet_status_handle.unsubscribe();

    println!("Stopping ActiveSession subscription...");
    active_session_handle.unsubscribe();

    // Alternative: Stop all subscriptions at once
    // manager.stop_all().await?;

    println!("All subscriptions stopped");

    // Final status check
    sleep(Duration::from_millis(100)).await;
    println!("Final active count: {}", manager.active_count().await);

    println!("\nSubscription System Demo Complete!");
    println!("=================================");
    println!("This demo shows that the Rust SDK provides:");
    println!("  - Complete WebSocket subscription system");
    println!("  - All 4 subscription types from JavaScript SDK");
    println!("  - Proper subscription lifecycle management");
    println!("  - Real-time event streaming with callbacks");
    println!("  - Connection management and error handling");
    println!("  - Multiple concurrent subscriptions");
    println!("  - Subscription status monitoring");

    Ok(())
}

/// Example of using subscriptions with the lower-level API directly
#[allow(dead_code)]
async fn advanced_subscription_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nAdvanced Subscription Example");
    println!("=============================");

    // Create GraphQL client directly (note: new() does not return Result)
    let graphql_client = Arc::new(GraphQLClient::new("ws://localhost:8080"));

    // Create subscription manager
    let manager = SubscriptionManager::new(graphql_client.clone());

    // Create subscription instances directly using the Subscribe trait
    let create_molecule_sub = CreateMoleculeSubscribe::new(graphql_client.clone());
    let active_wallet_sub = ActiveWalletSubscribe::new(graphql_client.clone());
    let wallet_status_sub = WalletStatusSubscribe::new(graphql_client.clone());
    let active_session_sub = ActiveSessionSubscribe::new(graphql_client);

    // Execute subscriptions with variables and callbacks
    let _handle1 = create_molecule_sub.execute(
        json!({"bundle": "example-bundle"}),
        Box::new(|data: Value| println!("Molecule: {}", data)),
    ).await?;

    let _handle2 = wallet_status_sub.execute(
        json!({"bundle": "example-bundle", "token": "KNISH"}),
        Box::new(|data: Value| println!("Wallet: {}", data)),
    ).await?;

    let _handle3 = active_wallet_sub.execute(
        json!({"bundle": "example-bundle"}),
        Box::new(|data: Value| println!("Active: {}", data)),
    ).await?;

    let _handle4 = active_session_sub.execute(
        json!({"metaType": "user", "metaId": "user456"}),
        Box::new(|data: Value| println!("Session: {}", data)),
    ).await?;

    println!("Advanced subscriptions configured");

    // Unsubscribe all at once via the manager
    manager.stop_all().await?;

    Ok(())
}

/// Example of error handling in subscriptions
#[allow(dead_code)]
async fn error_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nError Handling Example");
    println!("=====================");

    let client = KnishIOClient::new(
        "ws://invalid-endpoint:9999".to_string(),  // Invalid endpoint
        None,
        None,
        None,
        None,
        Some(true),
    );

    // This should handle connection errors gracefully
    match client.subscribe_create_molecule(
        Some("test-bundle".to_string()),
        |event: SubscriptionEvent| {
            println!("Event received: {} - {}", event.operation_name, event.data);
        }
    ).await {
        Ok(handle) => {
            println!("Subscription created: {}", handle.operation_name);
            // Clean up
            handle.unsubscribe();
        }
        Err(e) => {
            println!("Expected error caught: {}", e);
        }
    }

    Ok(())
}
