//! Example demonstrating the complete WebSocket subscription system
//!
//! This example shows how to use all 4 subscription types in the Rust SDK
//! to match the functionality available in the JavaScript SDK.

use knishio_client::{
    KnishIOClient, 
    CreateMoleculeSubscribe, ActiveWalletSubscribe, ActiveSessionSubscribe, WalletStatusSubscribe,
    SubscriptionManager, SubscriptionEvent,
    GraphQLClient
};
use std::collections::HashMap;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 KnishIO Rust SDK Subscription System Demo");
    println!("============================================");
    
    // Initialize the KnishIO client
    let client = KnishIOClient::new(
        "ws://localhost:8080",  // WebSocket endpoint
        Some("default".to_string()),    // Cell slug
        None,                          // Socket config (auto-generated)
        None,                          // GraphQL client (auto-generated)
        Some(3),                       // Server SDK version
        Some(true),                    // Enable logging
    );
    
    // Set a secret for the client session
    // client.set_secret("example-secret-for-testing-12345");
    
    println!("\n📡 Testing CreateMolecule Subscription");
    println!("-------------------------------------");
    
    // 1. CreateMolecule Subscription - Listen for new transactions
    let create_molecule_handle = client.subscribe_create_molecule(
        Some("test-bundle-hash".to_string()),
        |event: SubscriptionEvent| {
            if event.is_error() {
                println!("❌ CreateMolecule Error: {}", event.error.unwrap_or_default());
            } else {
                println!("✅ New Molecule Created: {}", event.data);
                if let Some(hash) = event.get_field("molecularHash") {
                    println!("   📄 Hash: {}", hash);
                }
                if let Some(status) = event.get_field("status") {
                    println!("   🔄 Status: {}", status);
                }
            }
        }
    ).await?;
    
    println!("✅ CreateMolecule subscription active: {}", create_molecule_handle.subscription_id);
    
    println!("\n👛 Testing ActiveWallet Subscription");
    println!("-----------------------------------");
    
    // 2. ActiveWallet Subscription - Listen for wallet updates
    let active_wallet_handle = client.subscribe_active_wallet(
        Some("test-bundle-hash".to_string()),
        |event: SubscriptionEvent| {
            if event.is_error() {
                println!("❌ ActiveWallet Error: {}", event.error.unwrap_or_default());
            } else {
                println!("✅ Wallet Update: {}", event.data);
                if let Some(address) = event.get_field("address") {
                    println!("   🏠 Address: {}", address);
                }
                if let Some(balance) = event.get_field("amount") {
                    println!("   💰 Balance: {}", balance);
                }
            }
        }
    ).await?;
    
    println!("✅ ActiveWallet subscription active: {}", active_wallet_handle.subscription_id);
    
    println!("\n📊 Testing WalletStatus Subscription");
    println!("-----------------------------------");
    
    // 3. WalletStatus Subscription - Monitor wallet status changes
    let wallet_status_handle = client.subscribe_wallet_status(
        Some("test-bundle-hash".to_string()),
        "TEST".to_string(),  // Token type
        |event: SubscriptionEvent| {
            if event.is_error() {
                println!("❌ WalletStatus Error: {}", event.error.unwrap_or_default());
            } else {
                println!("✅ Wallet Status Change: {}", event.data);
                if let Some(balance) = event.get_field("balance") {
                    println!("   💰 New Balance: {}", balance);
                }
                if let Some(admission) = event.get_field("admission") {
                    println!("   🎫 Admission: {}", admission);
                }
            }
        }
    ).await?;
    
    println!("✅ WalletStatus subscription active: {}", wallet_status_handle.subscription_id);
    
    println!("\n👥 Testing ActiveSession Subscription");
    println!("------------------------------------");
    
    // 4. ActiveSession Subscription - Monitor user activity
    let active_session_handle = client.subscribe_active_session(
        "user".to_string(),      // Meta type
        "user123".to_string(),   // Meta ID
        |event: SubscriptionEvent| {
            if event.is_error() {
                println!("❌ ActiveSession Error: {}", event.error.unwrap_or_default());
            } else {
                println!("✅ Session Activity: {}", event.data);
                if let Some(bundle) = event.get_field("bundleHash") {
                    println!("   👤 User: {}", bundle);
                }
                if let Some(updated) = event.get_field("updatedAt") {
                    println!("   🕐 Last Activity: {}", updated);
                }
            }
        }
    ).await?;
    
    println!("✅ ActiveSession subscription active: {}", active_session_handle.subscription_id);
    
    println!("\n🔍 Subscription Manager Status");
    println!("=============================");
    
    // Get subscription manager status
    let manager = client.get_subscription_manager()?;
    println!("📊 Active subscriptions: {}", manager.active_count().await);
    println!("🔗 WebSocket connected: {}", manager.is_connected().await);
    
    // List all subscription IDs
    let subscription_ids = manager.list_subscriptions().await;
    println!("📝 Subscription IDs:");
    for id in &subscription_ids {
        println!("   - {}", id);
    }
    
    println!("\n⏳ Running subscriptions for 30 seconds...");
    println!("   (In a real application, subscriptions would receive actual events)");
    
    // Let subscriptions run for a bit
    for i in 1..=6 {
        sleep(Duration::from_secs(5)).await;
        println!("⏱️  {}s elapsed... subscriptions still active", i * 5);
        
        // Check subscription status
        println!("   📊 Active count: {}", manager.active_count().await);
    }
    
    println!("\n🛑 Stopping All Subscriptions");
    println!("============================");
    
    // Stop individual subscriptions
    println!("Stopping CreateMolecule subscription...");
    create_molecule_handle.stop().await?;
    
    println!("Stopping ActiveWallet subscription...");
    active_wallet_handle.stop().await?;
    
    println!("Stopping WalletStatus subscription...");
    wallet_status_handle.stop().await?;
    
    println!("Stopping ActiveSession subscription...");
    active_session_handle.stop().await?;
    
    // Alternative: Stop all subscriptions at once
    // manager.stop_all().await?;
    
    println!("✅ All subscriptions stopped");
    
    // Final status check
    sleep(Duration::from_millis(100)).await;
    println!("📊 Final active count: {}", manager.active_count().await);
    
    println!("\n🎉 Subscription System Demo Complete!");
    println!("=====================================");
    println!("This demo shows that the Rust SDK provides:");
    println!("✅ Complete WebSocket subscription system");
    println!("✅ All 4 subscription types from JavaScript SDK");
    println!("✅ Proper subscription lifecycle management");
    println!("✅ Real-time event streaming with callbacks");
    println!("✅ Connection management and error handling");
    println!("✅ Multiple concurrent subscriptions");
    println!("✅ Subscription status monitoring");
    
    Ok(())
}

/// Example of using subscriptions with custom GraphQL client
#[allow(dead_code)]
async fn advanced_subscription_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 Advanced Subscription Example");
    println!("===============================");
    
    // Create GraphQL client directly
    let graphql_client = GraphQLClient::new("ws://localhost:8080")?;
    
    // Create subscription manager
    let manager = SubscriptionManager::new(graphql_client.clone());
    
    // Create subscription instances directly
    let create_molecule_sub = CreateMoleculeSubscribe::with_manager(
        graphql_client.clone(), 
        std::sync::Arc::new(manager)
    );
    
    let active_wallet_sub = ActiveWalletSubscribe::new(graphql_client.clone());
    let wallet_status_sub = WalletStatusSubscribe::new(graphql_client.clone());
    let active_session_sub = ActiveSessionSubscribe::new(graphql_client);
    
    // Set up subscription variables
    let mut create_molecule_vars = HashMap::new();
    create_molecule_vars.insert("bundle".to_string(), json!("example-bundle"));
    
    let mut wallet_status_vars = HashMap::new();
    wallet_status_vars.insert("bundle".to_string(), json!("example-bundle"));
    wallet_status_vars.insert("token".to_string(), json!("KNISH"));
    
    let mut active_wallet_vars = HashMap::new();
    active_wallet_vars.insert("bundle".to_string(), json!("example-bundle"));
    
    let mut active_session_vars = HashMap::new();
    active_session_vars.insert("metaType".to_string(), json!("user"));
    active_session_vars.insert("metaId".to_string(), json!("user456"));
    
    // Execute subscriptions with custom callbacks
    let _handle1 = create_molecule_sub.execute(
        Some(create_molecule_vars),
        |event| println!("🔄 Molecule: {}", event.data)
    ).await?;
    
    let _handle2 = wallet_status_sub.execute(
        Some(wallet_status_vars), 
        |event| println!("💰 Wallet: {}", event.data)
    ).await?;
    
    let _handle3 = active_wallet_sub.execute(
        Some(active_wallet_vars),
        |event| println!("👛 Active: {}", event.data)
    ).await?;
    
    let _handle4 = active_session_sub.execute(
        Some(active_session_vars),
        |event| println!("👥 Session: {}", event.data)
    ).await?;
    
    println!("✅ Advanced subscriptions configured");
    
    Ok(())
}

/// Example of error handling in subscriptions
#[allow(dead_code)]
async fn error_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚨 Error Handling Example");
    println!("========================");
    
    let client = KnishIOClient::new(
        "ws://invalid-endpoint:9999",  // Invalid endpoint
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
            if event.is_error() {
                println!("❌ Connection error handled: {}", event.error.unwrap_or_default());
            } else {
                println!("✅ Unexpected success: {}", event.data);
            }
        }
    ).await {
        Ok(handle) => {
            println!("✅ Subscription created despite invalid endpoint: {}", handle.subscription_id);
            // In the real implementation, this would attempt connection and handle errors
            let _status = handle.status().await;
        }
        Err(e) => {
            println!("❌ Expected error caught: {}", e);
        }
    }
    
    Ok(())
}