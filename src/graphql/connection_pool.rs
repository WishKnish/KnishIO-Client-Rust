//! Connection pooling and management for GraphQL HTTP clients
//!
//! This module provides connection pooling functionality to efficiently manage
//! HTTP connections to GraphQL endpoints.

use crate::error::{KnishIOError, Result};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Connection pool for managing HTTP clients to different endpoints
#[derive(Clone)]
pub struct ConnectionPool {
    clients: Arc<RwLock<HashMap<String, PooledClient>>>,
    config: PoolConfig,
}

/// Configuration for connection pool behavior
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of clients per endpoint
    pub max_clients_per_endpoint: usize,
    /// Client idle timeout
    pub idle_timeout: Duration,
    /// Connection timeout for new clients
    pub connect_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Keep-alive settings
    pub keep_alive_timeout: Duration,
    /// Pool cleanup interval
    pub cleanup_interval: Duration,
    /// User agent string
    pub user_agent: String,
}

/// A pooled HTTP client with metadata
#[derive(Debug, Clone)]
struct PooledClient {
    client: Client,
    created_at: Instant,
    last_used: Instant,
    request_count: u64,
}

/// Statistics for a connection pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_clients: usize,
    pub active_endpoints: usize,
    pub total_requests: u64,
    pub oldest_client_age: Option<Duration>,
    pub newest_client_age: Option<Duration>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        PoolConfig {
            max_clients_per_endpoint: 10,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(60),
            keep_alive_timeout: Duration::from_secs(90),
            cleanup_interval: Duration::from_secs(60),
            user_agent: format!("KnishIO-Rust-SDK/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl ConnectionPool {
    /// Create a new connection pool with default configuration
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }
    
    /// Create a new connection pool with custom configuration
    pub fn with_config(config: PoolConfig) -> Self {
        let pool = ConnectionPool {
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
        };
        
        // Start cleanup task
        pool.start_cleanup_task();
        
        pool
    }
    
    /// Get or create a client for the specified endpoint
    pub async fn get_client(&self, endpoint: &str) -> Result<Client> {
        let mut clients = self.clients.write().await;
        
        // Check if we have an existing client that's still valid
        if let Some(pooled_client) = clients.get_mut(endpoint) {
            let age = pooled_client.last_used.elapsed();
            
            if age < self.config.idle_timeout {
                pooled_client.last_used = Instant::now();
                pooled_client.request_count += 1;
                return Ok(pooled_client.client.clone());
            } else {
                // Client is too old, remove it
                clients.remove(endpoint);
            }
        }
        
        // Create new client
        let client = self.create_new_client()?;
        let pooled_client = PooledClient {
            client: client.clone(),
            created_at: Instant::now(),
            last_used: Instant::now(),
            request_count: 1,
        };
        
        clients.insert(endpoint.to_string(), pooled_client);
        
        debug!("Created new HTTP client for endpoint: {}", endpoint);
        
        Ok(client)
    }
    
    /// Create a new HTTP client with the pool's configuration
    fn create_new_client(&self) -> Result<Client> {
        let client = Client::builder()
            .timeout(self.config.request_timeout)
            .connect_timeout(self.config.connect_timeout)
            .pool_idle_timeout(self.config.keep_alive_timeout)
            .user_agent(&self.config.user_agent)
            .build()
            .map_err(|e| KnishIOError::custom(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(client)
    }
    
    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let clients = self.clients.read().await;
        let now = Instant::now();
        
        let mut total_requests = 0;
        let mut oldest_age = None;
        let mut newest_age = None;
        
        for pooled_client in clients.values() {
            total_requests += pooled_client.request_count;
            
            let age = now.duration_since(pooled_client.created_at);
            
            if oldest_age.is_none() || Some(age) > oldest_age {
                oldest_age = Some(age);
            }
            
            if newest_age.is_none() || Some(age) < newest_age {
                newest_age = Some(age);
            }
        }
        
        PoolStats {
            total_clients: clients.len(),
            active_endpoints: clients.len(), // Same in this implementation
            total_requests,
            oldest_client_age: oldest_age,
            newest_client_age: newest_age,
        }
    }
    
    /// Remove idle clients from the pool
    pub async fn cleanup(&self) {
        let mut clients = self.clients.write().await;
        let now = Instant::now();
        let mut removed_count = 0;
        
        clients.retain(|endpoint, pooled_client| {
            let age = now.duration_since(pooled_client.last_used);
            let should_keep = age < self.config.idle_timeout;
            
            if !should_keep {
                removed_count += 1;
                debug!("Removed idle client for endpoint: {}", endpoint);
            }
            
            should_keep
        });
        
        if removed_count > 0 {
            info!("Connection pool cleanup: removed {} idle clients", removed_count);
        }
    }
    
    /// Clear all clients from the pool
    pub async fn clear(&self) {
        let mut clients = self.clients.write().await;
        let count = clients.len();
        clients.clear();
        
        if count > 0 {
            info!("Cleared {} clients from connection pool", count);
        }
    }
    
    /// Get the number of clients in the pool
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
    
    /// Start the background cleanup task
    fn start_cleanup_task(&self) {
        let pool = self.clone();
        let cleanup_interval = self.config.cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                pool.cleanup().await;
            }
        });
    }
    
    /// Update configuration (affects new clients only)
    pub fn update_config(&mut self, new_config: PoolConfig) {
        self.config = new_config;
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &PoolConfig {
        &self.config
    }
    
    /// Check if an endpoint has an active client
    pub async fn has_client(&self, endpoint: &str) -> bool {
        let clients = self.clients.read().await;
        
        if let Some(pooled_client) = clients.get(endpoint) {
            let age = pooled_client.last_used.elapsed();
            age < self.config.idle_timeout
        } else {
            false
        }
    }
    
    /// Remove a specific client from the pool
    pub async fn remove_client(&self, endpoint: &str) -> bool {
        let mut clients = self.clients.write().await;
        clients.remove(endpoint).is_some()
    }
    
    /// Get list of active endpoints
    pub async fn get_active_endpoints(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }
}

/// Create a global connection pool instance
static GLOBAL_POOL: std::sync::LazyLock<ConnectionPool> = std::sync::LazyLock::new(|| {
    ConnectionPool::new()
});

/// Get the global connection pool
pub fn global_pool() -> &'static ConnectionPool {
    &GLOBAL_POOL
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    
    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_clients_per_endpoint, 10);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert!(config.user_agent.contains("KnishIO-Rust-SDK"));
    }
    
    #[tokio::test]
    async fn test_pool_creation() {
        let pool = ConnectionPool::new();
        assert_eq!(pool.client_count().await, 0);
        
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_clients, 0);
        assert_eq!(stats.total_requests, 0);
    }
    
    #[tokio::test]
    async fn test_client_creation_and_reuse() {
        let pool = ConnectionPool::new();
        let endpoint = "https://test.example.com/graphql";
        
        // First request should create a new client
        let _client1 = pool.get_client(endpoint).await.unwrap();
        assert_eq!(pool.client_count().await, 1);

        // Second request should reuse the existing client
        let _client2 = pool.get_client(endpoint).await.unwrap();
        assert_eq!(pool.client_count().await, 1);
        
        // Clients should be the same instance (Arc-cloned)
        // Note: We can't directly compare clients, but we can check pool state
        
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_clients, 1);
        assert_eq!(stats.total_requests, 2); // Two get_client calls
    }
    
    #[tokio::test]
    async fn test_client_cleanup() {
        let mut config = PoolConfig::default();
        config.idle_timeout = Duration::from_millis(100); // Very short timeout for testing
        
        let pool = ConnectionPool::with_config(config);
        let endpoint = "https://test.example.com/graphql";
        
        // Create a client
        let _client = pool.get_client(endpoint).await.unwrap();
        assert_eq!(pool.client_count().await, 1);
        
        // Wait for it to become idle
        sleep(Duration::from_millis(150)).await;
        
        // Manual cleanup should remove it
        pool.cleanup().await;
        assert_eq!(pool.client_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_multiple_endpoints() {
        let pool = ConnectionPool::new();
        
        let endpoint1 = "https://test1.example.com/graphql";
        let endpoint2 = "https://test2.example.com/graphql";
        
        let _client1 = pool.get_client(endpoint1).await.unwrap();
        let _client2 = pool.get_client(endpoint2).await.unwrap();
        
        assert_eq!(pool.client_count().await, 2);
        
        let endpoints = pool.get_active_endpoints().await;
        assert_eq!(endpoints.len(), 2);
        assert!(endpoints.contains(&endpoint1.to_string()));
        assert!(endpoints.contains(&endpoint2.to_string()));
    }
    
    #[tokio::test]
    async fn test_client_removal() {
        let pool = ConnectionPool::new();
        let endpoint = "https://test.example.com/graphql";
        
        let _client = pool.get_client(endpoint).await.unwrap();
        assert_eq!(pool.client_count().await, 1);
        assert!(pool.has_client(endpoint).await);
        
        let removed = pool.remove_client(endpoint).await;
        assert!(removed);
        assert_eq!(pool.client_count().await, 0);
        assert!(!pool.has_client(endpoint).await);
        
        // Removing non-existent client should return false
        let removed_again = pool.remove_client(endpoint).await;
        assert!(!removed_again);
    }
    
    #[tokio::test]
    async fn test_pool_clear() {
        let pool = ConnectionPool::new();
        
        // Add multiple clients
        let _client1 = pool.get_client("https://test1.example.com/graphql").await.unwrap();
        let _client2 = pool.get_client("https://test2.example.com/graphql").await.unwrap();
        let _client3 = pool.get_client("https://test3.example.com/graphql").await.unwrap();
        
        assert_eq!(pool.client_count().await, 3);
        
        pool.clear().await;
        assert_eq!(pool.client_count().await, 0);
    }
    
    #[test]
    fn test_global_pool() {
        let pool1 = global_pool();
        let pool2 = global_pool();
        
        // Should be the same instance
        assert!(std::ptr::eq(pool1, pool2));
    }
}