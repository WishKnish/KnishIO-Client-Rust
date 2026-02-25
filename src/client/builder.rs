//! ClientBuilder for fluent KnishIOClient configuration
//!
//! Provides a type-safe, builder-pattern API for configuring and creating KnishIOClient instances.
//! Follows Rust 2025 best practices with compile-time validation, zero-cost abstractions,
//! and comprehensive error handling.
//!
//! # Examples
//!
//! ```rust
//! use knishio_client::client::ClientBuilder;
//!
//! // Basic client
//! let client = ClientBuilder::new()
//!     .uri("https://api.knish.io")
//!     .secret("my-secret-key")
//!     .build()?;
//!
//! // Advanced configuration
//! let client = ClientBuilder::new()
//!     .uris(vec!["https://node1.knish.io", "https://node2.knish.io"])
//!     .secret("my-secret-key") 
//!     .cell_slug("my-cell")
//!     .encryption(true)
//!     .logging(true)
//!     .server_sdk_version(3)
//!     .build()?;
//! ```

use crate::client::KnishIOClient;
use crate::graphql::{GraphQLClient, SocketConfig};
use crate::error::{KnishIOError, Result};
use std::collections::HashMap;

/// Builder for creating KnishIOClient instances with fluent API
///
/// Provides compile-time validation and type-safe configuration of all client options.
/// Uses the builder pattern to ensure required fields are set and optional fields have
/// sensible defaults.
#[derive(Clone)]
pub struct ClientBuilder {
    /// List of node URIs (at least one required)
    uris: Vec<String>,
    /// Optional cell slug for targeting specific sub-ledgers
    cell_slug: Option<String>,
    /// User secret for cryptographic operations
    secret: Option<String>,
    /// WebSocket configuration for real-time subscriptions
    socket_config: Option<SocketConfig>,
    /// Custom GraphQL client (optional)
    graphql_client: Option<GraphQLClient>,
    /// Server SDK version compatibility
    server_sdk_version: u32,
    /// Enable ML-KEM quantum encryption
    encryption: bool,
    /// Enable debug logging
    logging: bool,
    /// Connection timeout in seconds
    connection_timeout: Option<u64>,
    /// Request timeout in seconds
    request_timeout: Option<u64>,
    /// Custom headers for requests
    custom_headers: HashMap<String, String>,
    /// Retry configuration
    max_retries: Option<u32>,
    /// Enable automatic authentication
    auto_auth: bool,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    /// Create a new ClientBuilder with default configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use knishio_client::client::ClientBuilder;
    ///
    /// let builder = ClientBuilder::new();
    /// ```
    pub fn new() -> Self {
        ClientBuilder {
            uris: Vec::new(),
            cell_slug: None,
            secret: None,
            socket_config: None,
            graphql_client: None,
            server_sdk_version: 3, // Default to SDK version 3
            encryption: false,
            logging: false,
            connection_timeout: None,
            request_timeout: None,
            custom_headers: HashMap::new(),
            max_retries: None,
            auto_auth: true, // Enable auto-auth by default
        }
    }

    /// Set a single URI for the client
    ///
    /// # Arguments
    ///
    /// * `uri` - Node URI (e.g., "https://api.knish.io")
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().uri("https://api.knish.io");
    /// ```
    pub fn uri<S: Into<String>>(mut self, uri: S) -> Self {
        self.uris = vec![uri.into()];
        self
    }

    /// Set multiple URIs for load balancing
    ///
    /// # Arguments
    ///
    /// * `uris` - Vector of node URIs
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new()
    ///     .uris(vec!["https://node1.knish.io", "https://node2.knish.io"]);
    /// ```
    pub fn uris<S: Into<String>>(mut self, uris: Vec<S>) -> Self {
        self.uris = uris.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add an additional URI to the existing list
    ///
    /// # Arguments
    ///
    /// * `uri` - Additional node URI
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new()
    ///     .uri("https://node1.knish.io")
    ///     .add_uri("https://node2.knish.io");
    /// ```
    pub fn add_uri<S: Into<String>>(mut self, uri: S) -> Self {
        self.uris.push(uri.into());
        self
    }

    /// Set the user secret for cryptographic operations
    ///
    /// # Arguments
    ///
    /// * `secret` - User secret key
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().secret("my-secret-key");
    /// ```
    pub fn secret<S: Into<String>>(mut self, secret: S) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Set the cell slug for targeting specific sub-ledgers
    ///
    /// # Arguments
    ///
    /// * `cell_slug` - Cell identifier
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().cell_slug("my-cell");
    /// ```
    pub fn cell_slug<S: Into<String>>(mut self, cell_slug: S) -> Self {
        self.cell_slug = Some(cell_slug.into());
        self
    }

    /// Enable or disable ML-KEM quantum encryption
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable encryption
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().encryption(true);
    /// ```
    pub fn encryption(mut self, enabled: bool) -> Self {
        self.encryption = enabled;
        self
    }

    /// Enable or disable debug logging
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable logging
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().logging(true);
    /// ```
    pub fn logging(mut self, enabled: bool) -> Self {
        self.logging = enabled;
        self
    }

    /// Set the server SDK version for compatibility
    ///
    /// # Arguments
    ///
    /// * `version` - SDK version (2 or 3)
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().server_sdk_version(3);
    /// ```
    pub fn server_sdk_version(mut self, version: u32) -> Self {
        self.server_sdk_version = version;
        self
    }

    /// Set the connection timeout
    ///
    /// # Arguments
    ///
    /// * `timeout_seconds` - Connection timeout in seconds
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().connection_timeout(30);
    /// ```
    pub fn connection_timeout(mut self, timeout_seconds: u64) -> Self {
        self.connection_timeout = Some(timeout_seconds);
        self
    }

    /// Set the request timeout
    ///
    /// # Arguments
    ///
    /// * `timeout_seconds` - Request timeout in seconds
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().request_timeout(60);
    /// ```
    pub fn request_timeout(mut self, timeout_seconds: u64) -> Self {
        self.request_timeout = Some(timeout_seconds);
        self
    }

    /// Add a custom header to all requests
    ///
    /// # Arguments
    ///
    /// * `key` - Header name
    /// * `value` - Header value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new()
    ///     .custom_header("X-Client-Version", "1.0.0");
    /// ```
    pub fn custom_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.custom_headers.insert(key.into(), value.into());
        self
    }

    /// Set multiple custom headers
    ///
    /// # Arguments
    ///
    /// * `headers` - HashMap of headers
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// 
    /// let mut headers = HashMap::new();
    /// headers.insert("X-Client-Version".to_string(), "1.0.0".to_string());
    /// 
    /// let builder = ClientBuilder::new().custom_headers(headers);
    /// ```
    pub fn custom_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.custom_headers.extend(headers);
        self
    }

    /// Set the maximum number of retries for failed requests
    ///
    /// # Arguments
    ///
    /// * `retries` - Maximum retry attempts
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().max_retries(3);
    /// ```
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Enable or disable automatic authentication
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable auto-authentication
    ///
    /// # Examples
    ///
    /// ```rust
    /// let builder = ClientBuilder::new().auto_auth(false);
    /// ```
    pub fn auto_auth(mut self, enabled: bool) -> Self {
        self.auto_auth = enabled;
        self
    }

    /// Configure WebSocket settings for real-time subscriptions
    ///
    /// # Arguments
    ///
    /// * `socket_config` - WebSocket configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use knishio_client::graphql::SocketConfig;
    /// 
    /// let socket_config = SocketConfig::default();
    /// let builder = ClientBuilder::new().socket_config(socket_config);
    /// ```
    pub fn socket_config(mut self, config: SocketConfig) -> Self {
        self.socket_config = Some(config);
        self
    }

    /// Use a custom GraphQL client
    ///
    /// # Arguments
    ///
    /// * `client` - Custom GraphQL client instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use knishio_client::graphql::GraphQLClient;
    /// 
    /// let client = GraphQLClient::new("https://api.knish.io");
    /// let builder = ClientBuilder::new().graphql_client(client);
    /// ```
    pub fn graphql_client(mut self, client: GraphQLClient) -> Self {
        self.graphql_client = Some(client);
        self
    }

    /// Validate the builder configuration
    ///
    /// # Returns
    ///
    /// Result indicating validation success or specific error
    fn validate(&self) -> Result<()> {
        // Check that at least one URI is provided
        if self.uris.is_empty() {
            return Err(KnishIOError::ConfigurationError("At least one URI must be specified".into()));
        }

        // Validate URIs format
        for uri in &self.uris {
            if !uri.starts_with("http://") && !uri.starts_with("https://") && !uri.starts_with("ws://") && !uri.starts_with("wss://") {
                return Err(KnishIOError::ConfigurationError(format!("Invalid URI format: {}", uri)));
            }
        }

        // Validate SDK version
        if self.server_sdk_version < 2 || self.server_sdk_version > 4 {
            return Err(KnishIOError::ConfigurationError("Server SDK version must be between 2 and 4".into()));
        }

        // Validate timeout values
        if let Some(timeout) = self.connection_timeout {
            if timeout == 0 || timeout > 300 {
                return Err(KnishIOError::ConfigurationError("Connection timeout must be between 1 and 300 seconds".into()));
            }
        }

        if let Some(timeout) = self.request_timeout {
            if timeout == 0 || timeout > 600 {
                return Err(KnishIOError::ConfigurationError("Request timeout must be between 1 and 600 seconds".into()));
            }
        }

        // Validate retry count
        if let Some(retries) = self.max_retries {
            if retries > 10 {
                return Err(KnishIOError::ConfigurationError("Maximum retries cannot exceed 10".into()));
            }
        }

        Ok(())
    }

    /// Build the KnishIOClient with the configured settings
    ///
    /// # Returns
    ///
    /// Result containing the configured KnishIOClient or validation error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use knishio_client::client::ClientBuilder;
    ///
    /// let client = ClientBuilder::new()
    ///     .uri("https://api.knish.io")
    ///     .secret("my-secret")
    ///     .build()?;
    /// ```
    pub fn build(self) -> Result<KnishIOClient> {
        // Validate configuration
        self.validate()?;

        // Create the client with validated configuration
        let mut client = KnishIOClient::new(
            self.uris.clone(),
            self.cell_slug.clone(),
            self.socket_config.clone(),
            self.graphql_client.clone(),
            Some(self.server_sdk_version),
            Some(self.logging),
        );

        // Set the secret if provided
        if let Some(secret) = self.secret {
            client.set_secret(secret);
        }

        // Apply encryption setting
        client.set_encrypt(self.encryption);

        // TODO: Apply additional configuration like timeouts, headers, retries
        // These would need corresponding methods on KnishIOClient

        Ok(client)
    }

    /// Build the client asynchronously and perform initial setup
    ///
    /// # Returns
    ///
    /// Result containing the configured and initialized KnishIOClient
    ///
    /// # Examples
    ///
    /// ```rust
    /// use knishio_client::client::ClientBuilder;
    ///
    /// let client = ClientBuilder::new()
    ///     .uri("https://api.knish.io")
    ///     .secret("my-secret")
    ///     .build_async().await?;
    /// ```
    pub async fn build_async(self) -> Result<KnishIOClient> {
        // Save values before self is moved
        let auto_auth = self.auto_auth;
        let logging = self.logging;
        
        let mut client = self.build()?;

        // Perform initial setup if auto-auth is enabled
        if auto_auth && client.has_secret() {
            // Attempt initial authentication
            match client.ensure_authentication(None).await {
                Ok(_) => {
                    if logging {
                        eprintln!("[ClientBuilder] Initial authentication successful");
                    }
                }
                Err(e) => {
                    if logging {
                        eprintln!("[ClientBuilder] Initial authentication failed: {}", e);
                    }
                    // Don't fail the build, just log the issue
                }
            }
        }

        Ok(client)
    }
}

/// Type-safe configuration presets for common use cases
impl ClientBuilder {
    /// Create a production-ready client configuration
    ///
    /// # Arguments
    ///
    /// * `uri` - Production node URI
    /// * `secret` - User secret key
    ///
    /// # Examples
    ///
    /// ```rust
    /// let client = ClientBuilder::production("https://api.knish.io", "my-secret").build()?;
    /// ```
    pub fn production<S: Into<String>>(uri: S, secret: S) -> Self {
        Self::new()
            .uri(uri)
            .secret(secret)
            .server_sdk_version(3)
            .encryption(true)
            .logging(false)
            .connection_timeout(30)
            .request_timeout(60)
            .max_retries(3)
            .auto_auth(true)
    }

    /// Create a development-friendly client configuration
    ///
    /// # Arguments
    ///
    /// * `uri` - Development node URI
    /// * `secret` - User secret key
    ///
    /// # Examples
    ///
    /// ```rust
    /// let client = ClientBuilder::development("http://localhost:8000", "test-secret").build()?;
    /// ```
    pub fn development<S: Into<String>>(uri: S, secret: S) -> Self {
        Self::new()
            .uri(uri)
            .secret(secret)
            .server_sdk_version(3)
            .encryption(false)
            .logging(true)
            .connection_timeout(10)
            .request_timeout(30)
            .max_retries(1)
            .auto_auth(true)
    }

    /// Create a load-balanced client configuration
    ///
    /// # Arguments
    ///
    /// * `uris` - Multiple node URIs for load balancing
    /// * `secret` - User secret key
    ///
    /// # Examples
    ///
    /// ```rust
    /// let client = ClientBuilder::load_balanced(
    ///     vec!["https://node1.knish.io", "https://node2.knish.io"],
    ///     "my-secret"
    /// ).build()?;
    /// ```
    pub fn load_balanced<S: Into<String>>(uris: Vec<S>, secret: S) -> Self {
        Self::new()
            .uris(uris)
            .secret(secret)
            .server_sdk_version(3)
            .encryption(true)
            .logging(false)
            .connection_timeout(20)
            .request_timeout(45)
            .max_retries(2)
            .auto_auth(true)
    }

    /// Create a minimal client configuration for testing
    ///
    /// # Arguments
    ///
    /// * `uri` - Test node URI
    ///
    /// # Examples
    ///
    /// ```rust
    /// let client = ClientBuilder::minimal("http://localhost:8000").build()?;
    /// ```
    pub fn minimal<S: Into<String>>(uri: S) -> Self {
        Self::new()
            .uri(uri)
            .server_sdk_version(3)
            .encryption(false)
            .logging(false)
            .auto_auth(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = ClientBuilder::new();
        assert_eq!(builder.uris.len(), 0);
        assert_eq!(builder.server_sdk_version, 3);
        assert!(!builder.encryption);
        assert!(!builder.logging);
        assert!(builder.auto_auth);
    }

    #[test]
    fn test_builder_fluent_api() {
        let builder = ClientBuilder::new()
            .uri("https://api.knish.io")
            .secret("test-secret")
            .cell_slug("test-cell")
            .encryption(true)
            .logging(true)
            .server_sdk_version(3);

        assert_eq!(builder.uris, vec!["https://api.knish.io"]);
        assert_eq!(builder.secret, Some("test-secret".to_string()));
        assert_eq!(builder.cell_slug, Some("test-cell".to_string()));
        assert!(builder.encryption);
        assert!(builder.logging);
        assert_eq!(builder.server_sdk_version, 3);
    }

    #[test]
    fn test_builder_multiple_uris() {
        let builder = ClientBuilder::new()
            .uris(vec!["https://node1.knish.io", "https://node2.knish.io"]);

        assert_eq!(builder.uris.len(), 2);
        assert!(builder.uris.contains(&"https://node1.knish.io".to_string()));
        assert!(builder.uris.contains(&"https://node2.knish.io".to_string()));
    }

    #[test]
    fn test_builder_add_uri() {
        let builder = ClientBuilder::new()
            .uri("https://node1.knish.io")
            .add_uri("https://node2.knish.io");

        assert_eq!(builder.uris.len(), 2);
    }

    #[test]
    fn test_validation_no_uri() {
        let builder = ClientBuilder::new();
        let result = builder.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("At least one URI"));
    }

    #[test]
    fn test_validation_invalid_uri() {
        let builder = ClientBuilder::new().uri("invalid-uri");
        let result = builder.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid URI format"));
    }

    #[test]
    fn test_validation_invalid_sdk_version() {
        let builder = ClientBuilder::new()
            .uri("https://api.knish.io")
            .server_sdk_version(10);
        let result = builder.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Server SDK version"));
    }

    #[test]
    fn test_presets_production() {
        let builder = ClientBuilder::production("https://api.knish.io", "secret");
        assert_eq!(builder.uris, vec!["https://api.knish.io"]);
        assert_eq!(builder.secret, Some("secret".to_string()));
        assert!(builder.encryption);
        assert!(!builder.logging);
        assert_eq!(builder.connection_timeout, Some(30));
        assert_eq!(builder.max_retries, Some(3));
    }

    #[test]
    fn test_presets_development() {
        let builder = ClientBuilder::development("http://localhost:8000", "test-secret");
        assert_eq!(builder.uris, vec!["http://localhost:8000"]);
        assert_eq!(builder.secret, Some("test-secret".to_string()));
        assert!(!builder.encryption);
        assert!(builder.logging);
        assert_eq!(builder.connection_timeout, Some(10));
        assert_eq!(builder.max_retries, Some(1));
    }
}