//! Production-Ready GraphQL Client for KnishIO
//!
//! This module provides a complete GraphQL communication layer for KnishIO nodes,
//! with full compatibility with the JavaScript URQL client wrapper.
//!
//! # Features
//!
//! - **HTTP/HTTPS Queries & Mutations**: Full GraphQL operation support
//! - **WebSocket Subscriptions**: Real-time data with auto-reconnection
//! - **Connection Pooling**: Efficient resource management
//! - **Retry Logic**: Exponential backoff with configurable policies
//! - **Authentication**: Seamless token-based auth with auto-refresh
//! - **Error Handling**: Comprehensive error propagation and recovery
//! - **Timeout Management**: Request-level and global timeout controls
//! - **Debug Logging**: Structured logging with tracing support
//!
//! # Architecture
//!
//! The client follows the same patterns as the JavaScript implementation:
//! - `GraphQLClient` (equivalent to UrqlClientWrapper)
//! - Connection management with auth token integration
//! - Response formatting to match JavaScript output
//! - WebSocket subscription handling

use crate::error::{KnishIOError, Result};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Sub-modules for advanced functionality
mod websocket;
mod connection_pool;
mod retry_policy;

// Re-export public types from sub-modules
pub use websocket::{
    WebSocketManager, ConnectionState, ReconnectConfig as WebSocketReconnectConfig
};
pub use connection_pool::{
    ConnectionPool, PoolConfig as ConnectionPoolConfig, PoolStats, global_pool
};
pub use retry_policy::{
    RetryPolicy, RetryStrategy, RetryCondition, RetryExecutor, execute_with_retry
};

/// The post-quantum `CipherHash` wrapper query. The validator intercepts this
/// operation, ML-KEM-decrypts `$Hash`, executes the inner request, and returns the
/// encrypted response in `{ data: { CipherHash: { hash } } }`. Byte-identical to
/// the string the other seven SDKs send.
const CIPHER_HASH_QUERY: &str = "query ( $Hash: String! ) { CipherHash ( Hash: $Hash ) { hash } }";

/// Light parse of a GraphQL body's operation type + root field name, for the
/// CipherHash bypass decision (no full GraphQL parse needed). Operation type is the
/// first `query`/`mutation`/`subscription` keyword (default `query` for an anonymous
/// `{ ... }`); root field is the first identifier inside the top-level selection set.
fn parse_operation(query: &str) -> (String, String) {
    let lowered = query.to_ascii_lowercase();
    let op_type = ["query", "mutation", "subscription"]
        .iter()
        .filter_map(|kw| {
            lowered.find(kw).and_then(|idx| {
                let before_ok = idx == 0
                    || !lowered.as_bytes()[idx - 1].is_ascii_alphanumeric();
                let after = idx + kw.len();
                let after_ok = after >= lowered.len()
                    || !lowered.as_bytes()[after].is_ascii_alphanumeric();
                (before_ok && after_ok).then_some((idx, (*kw).to_string()))
            })
        })
        .min_by_key(|(idx, _)| *idx)
        .map(|(_, kw)| kw)
        .unwrap_or_else(|| "query".to_string());

    let name = query
        .find('{')
        .map(|brace| &query[brace + 1..])
        .and_then(|rest| {
            let start = rest.find(|c: char| c.is_ascii_alphabetic() || c == '_')?;
            let tail = &rest[start..];
            let end = tail
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(tail.len());
            Some(tail[..end].to_string())
        })
        .unwrap_or_default();

    (op_type, name)
}

/// GraphQL request structure
#[derive(Debug, Clone, Serialize)]
pub struct GraphQLRequest {
    /// GraphQL query string
    pub query: Option<String>,
    /// GraphQL mutation string  
    pub mutation: Option<String>,
    /// Variables for the operation
    pub variables: Option<Value>,
    /// Optional operation name for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    /// Request timeout override
    #[serde(skip)]
    pub timeout: Option<Duration>,
    /// Request-specific headers
    #[serde(skip)]
    pub headers: HashMap<String, String>,
}

/// GraphQL response structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphQLResponse {
    /// Response data
    pub data: Option<Value>,
    /// GraphQL errors if any
    pub errors: Option<Vec<GraphQLError>>,
    /// Response extensions (server metadata)
    pub extensions: Option<Value>,
}

/// GraphQL error structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphQLError {
    /// Error message
    pub message: String,
    /// Error locations in the query
    pub locations: Option<Vec<ErrorLocation>>,
    /// Error path
    pub path: Option<Vec<Value>>,
    /// Error extensions (custom error data)
    pub extensions: Option<HashMap<String, Value>>,
}

/// GraphQL error location
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorLocation {
    /// Line number in the query
    pub line: u32,
    /// Column number in the query  
    pub column: u32,
}

/// WebSocket configuration for subscriptions
#[derive(Debug, Clone)]
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

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier (exponential backoff)
    pub backoff_multiplier: f64,
    /// Whether to retry on network errors
    pub retry_on_network_error: bool,
    /// Whether to retry on server errors (5xx)
    pub retry_on_server_error: bool,
}

/// GraphQL Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Maximum number of connections
    pub max_connections: usize,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Keep-alive timeout
    pub keep_alive_timeout: Duration,
    /// TCP keep-alive settings
    pub tcp_keepalive: Option<Duration>,
    /// Accept invalid TLS certificates (for self-signed certs in dev)
    pub insecure_tls: bool,
}

/// Subscription handle for managing active subscriptions
#[derive(Debug)]
pub struct SubscriptionHandle {
    /// Unique subscription ID
    pub id: String,
    /// Channel for sending stop signals
    pub stop_sender: mpsc::UnboundedSender<()>,
    /// Operation name for debugging
    pub operation_name: Option<String>,
}

/// Main GraphQL client wrapper (equivalent to UrqlClientWrapper)
#[derive(Clone)]
pub struct GraphQLClient {
    /// Server URI for GraphQL operations
    server_uri: String,
    /// WebSocket configuration for subscriptions
    socket_config: Option<SocketConfig>,
    /// Current authentication token
    auth_token: Option<String>,
    /// The validator's advertised ML-KEM public key (the CipherHash encryption
    /// target), learned at auth.
    pubkey: Option<String>,
    /// The client's AUTH wallet — holds the ML-KEM private key that decrypts
    /// CipherHash responses addressed to us.
    wallet: Option<crate::wallet::Wallet>,
    /// Whether to encrypt communications
    encrypt: bool,
    /// HTTP client with connection pooling
    http_client: Arc<Client>,
    /// Retry configuration
    #[allow(dead_code)]
    retry_config: RetryConfig,
    /// Active subscription handles
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionHandle>>>,
    /// Request timeout
    #[allow(dead_code)]
    request_timeout: Duration,
    /// Debug logging enabled
    #[allow(dead_code)]
    debug: bool,
}

impl Default for SocketConfig {
    fn default() -> Self {
        SocketConfig {
            socket_uri: String::new(),
            app_key: "knishio".to_string(),
            connect_timeout: Some(Duration::from_secs(10)),
            keep_alive_interval: Some(Duration::from_secs(30)),
            max_reconnect_attempts: Some(5),
            reconnect_delay: Some(Duration::from_secs(2)),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            retry_on_network_error: true,
            retry_on_server_error: true,
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            max_connections: 100,
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(60),
            keep_alive_timeout: Duration::from_secs(90),
            tcp_keepalive: Some(Duration::from_secs(60)),
            insecure_tls: false,
        }
    }
}

impl GraphQLClient {
    /// Create a new GraphQL client with default configuration
    pub fn new(server_uri: impl Into<String>) -> Self {
        Self::with_config(server_uri, ClientConfig::default(), RetryConfig::default())
    }
    
    /// Create a new GraphQL client with custom configuration
    pub fn with_config(
        server_uri: impl Into<String>,
        client_config: ClientConfig,
        retry_config: RetryConfig,
    ) -> Self {
        let mut builder = Client::builder()
            .timeout(client_config.request_timeout)
            .connect_timeout(client_config.connect_timeout)
            .pool_idle_timeout(client_config.keep_alive_timeout)
            .pool_max_idle_per_host(client_config.max_connections)
            .tcp_keepalive(client_config.tcp_keepalive)
            .user_agent(format!("KnishIO-Rust-SDK/{}", env!("CARGO_PKG_VERSION")));

        if client_config.insecure_tls {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let http_client = builder.build().unwrap_or_else(|e| {
            eprintln!("CRITICAL: Failed to create HTTP client: {}", e);
            Client::new()
        });

        GraphQLClient {
            server_uri: server_uri.into(),
            socket_config: None,
            auth_token: None,
            pubkey: None,
            wallet: None,
            encrypt: false,
            http_client: Arc::new(http_client),
            retry_config,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            request_timeout: client_config.request_timeout,
            debug: false,
        }
    }

    /// Create client with socket configuration for subscriptions
    pub fn with_socket(
        server_uri: impl Into<String>,
        socket_config: SocketConfig,
        encrypt: bool,
    ) -> Self {
        let mut client = Self::new(server_uri);
        client.socket_config = Some(socket_config);
        client.encrypt = encrypt;
        client
    }

    /// Set authentication data (equivalent to setAuthData in JS).
    ///
    /// `pubkey` is the validator's ML-KEM public key and `wallet` the client's AUTH
    /// wallet; both are required before the CipherHash transport can encrypt.
    pub fn set_auth_data(
        &mut self,
        token: String,
        pubkey: Option<String>,
        wallet: Option<crate::wallet::Wallet>,
    ) {
        self.auth_token = Some(token);
        self.pubkey = pubkey;
        self.wallet = wallet;
    }

    /// Set server URI
    pub fn set_uri(&mut self, uri: impl Into<String>) {
        self.server_uri = uri.into();
    }

    /// Get current server URI
    pub fn get_uri(&self) -> &str {
        &self.server_uri
    }

    /// Get socket URI if configured
    pub fn get_socket_uri(&self) -> Option<&str> {
        self.socket_config.as_ref().map(|config| config.socket_uri.as_str())
    }

    /// Set encryption mode
    pub fn set_encryption(&mut self, encrypt: bool) {
        self.encrypt = encrypt;
    }
    
    /// Get socket configuration
    pub fn get_socket_config(&self) -> Option<&SocketConfig> {
        self.socket_config.as_ref()
    }
    
    /// Get authentication token
    pub fn get_auth_token(&self) -> Option<String> {
        self.auth_token.clone()
    }

    /// Whether an outgoing GraphQL body should be wrapped in the CipherHash
    /// envelope. Bypass (plaintext): introspection `__schema`, `ContinuId`, the
    /// `AccessToken` mutation, and the U-isotope `ProposeMolecule` auth bootstrap —
    /// the key exchange itself cannot be encrypted. Mirrors the JS/Kotlin/validator
    /// bypass set exactly.
    fn should_encrypt(&self, payload: &Value) -> bool {
        let query = payload.get("query").and_then(Value::as_str).unwrap_or("");
        let (op_type, name) = parse_operation(query);

        if op_type == "query" && (name == "__schema" || name == "ContinuId") {
            return false;
        }
        if op_type == "mutation" && name == "AccessToken" {
            return false;
        }
        if op_type == "mutation" && name == "ProposeMolecule" {
            let isotope = payload
                .get("variables")
                .and_then(|v| v.get("molecule"))
                .and_then(|m| m.get("atoms"))
                .and_then(|a| a.get(0))
                .and_then(|a| a.get("isotope"))
                .and_then(Value::as_str);
            if isotope == Some("U") {
                return false;
            }
        }
        true
    }

    /// POST a GraphQL payload, wrapping it in the ML-KEM `CipherHash` envelope when
    /// encryption is enabled and the operation is not on the bypass list.
    ///
    /// Fails closed: if encryption is requested but no ML-KEM transport key is
    /// available, the request is refused rather than silently sent in plaintext.
    async fn send(&self, payload: Value) -> Result<GraphQLResponse> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            "application/json"
                .parse()
                .map_err(|_| KnishIOError::custom("Invalid Content-Type header"))?,
        );

        if let Some(token) = &self.auth_token {
            headers.insert(
                "X-Auth-Token",
                token
                    .parse()
                    .map_err(|_| KnishIOError::custom("Invalid auth token header"))?,
            );
        }

        let encrypt_this = self.encrypt && self.should_encrypt(&payload);

        let (body, wallet) = if encrypt_this {
            let wallet = self.wallet.as_ref().ok_or_else(|| {
                KnishIOError::custom(
                    "CipherHash: encryption is enabled but no ML-KEM transport key is available \
                     — authenticate first, or the validator advertised no ML-KEM pubkey",
                )
            })?;
            let server_pubkey = self.pubkey.as_deref().ok_or_else(|| {
                KnishIOError::custom(
                    "CipherHash: encryption is enabled but no ML-KEM transport key is available \
                     — authenticate first, or the validator advertised no ML-KEM pubkey",
                )
            })?;

            let hash_var = wallet
                .encrypt_string_ml(&payload.to_string(), server_pubkey)
                .await?;
            (
                json!({ "query": CIPHER_HASH_QUERY, "variables": { "Hash": hash_var } }),
                Some(wallet),
            )
        } else {
            (payload, None)
        };

        let response = self
            .http_client
            .post(&self.server_uri)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(KnishIOError::from_network_error)?;

        if !response.status().is_success() {
            return Err(KnishIOError::custom(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let text = response.text().await.map_err(KnishIOError::from_network_error)?;

        let graphql_response: GraphQLResponse = match wallet {
            Some(wallet) => {
                let parsed: Value = serde_json::from_str(&text)
                    .map_err(|e| KnishIOError::custom(format!("Invalid response JSON: {e}")))?;
                match parsed
                    .get("data")
                    .and_then(|d| d.get("CipherHash"))
                    .and_then(|c| c.get("hash"))
                    .and_then(Value::as_str)
                {
                    // Encrypted reply → decrypt back to the inner GraphQL response.
                    Some(hash) => {
                        let map: HashMap<String, crate::wallet::EncryptedMessage> =
                            serde_json::from_str(hash).map_err(|e| {
                                KnishIOError::custom(format!("CipherHash: malformed response map: {e}"))
                            })?;
                        let inner = wallet
                            .decrypt_my_message_ml(&map)
                            .await
                            .ok_or(KnishIOError::DecryptionKey)?;
                        serde_json::from_str(&inner).map_err(|e| {
                            KnishIOError::custom(format!("CipherHash: malformed inner response: {e}"))
                        })?
                    }
                    // Plaintext (e.g. a validator-side error) — pass through unchanged.
                    None => serde_json::from_str(&text)
                        .map_err(|e| KnishIOError::custom(format!("Invalid response JSON: {e}")))?,
                }
            }
            None => serde_json::from_str(&text)
                .map_err(|e| KnishIOError::custom(format!("Invalid response JSON: {e}")))?,
        };

        self.format_response(graphql_response)
    }

    /// Execute a GraphQL query
    pub async fn query(&self, request: GraphQLRequest) -> Result<GraphQLResponse> {
        self.send(json!({
            "query": request.query,
            "variables": request.variables,
            "operationName": request.operation_name
        })).await
    }

    /// Execute a GraphQL mutation
    pub async fn mutate(&self, request: GraphQLRequest) -> Result<GraphQLResponse> {
        self.send(json!({
            "query": request.mutation,
            "variables": request.variables,
            "operationName": request.operation_name
        })).await
    }

    /// Subscribe to GraphQL subscription (WebSocket-based)
    pub async fn subscribe<F>(&mut self, request: GraphQLRequest, mut callback: F) -> Result<()>
    where
        F: FnMut(GraphQLResponse) + Send + 'static,
    {
        let socket_config = self.socket_config.as_ref()
            .ok_or_else(|| KnishIOError::custom("Socket not configured for subscriptions"))?;

        let ws_url = &socket_config.socket_uri;
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| KnishIOError::custom(format!("WebSocket connection failed: {}", e)))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send connection init
        let init_message = json!({
            "type": "connection_init",
            "payload": {
                "authToken": self.auth_token
            }
        });
        
        ws_sender
            .send(Message::Text(init_message.to_string().into()))
            .await
            .map_err(|e| KnishIOError::custom(format!("Failed to send init: {}", e)))?;

        // Send subscription
        let sub_message = json!({
            "type": "start",
            "payload": {
                "query": request.query.or(request.mutation),
                "variables": request.variables
            }
        });

        ws_sender
            .send(Message::Text(sub_message.to_string().into()))
            .await
            .map_err(|e| KnishIOError::custom(format!("Failed to send subscription: {}", e)))?;

        // Handle incoming messages
        tokio::spawn(async move {
            while let Some(message) = ws_receiver.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(response) = serde_json::from_str::<GraphQLResponse>(&text) {
                            callback(response);
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
        });

        Ok(())
    }

    // Old methods removed - replaced with proper async subscription management

    /// Format response (equivalent to formatResponse in JS)
    fn format_response(&self, response: GraphQLResponse) -> Result<GraphQLResponse> {
        // Check for errors
        if let Some(ref errors) = response.errors {
            if !errors.is_empty() {
                let error_msg = errors.iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(KnishIOError::custom(format!("GraphQL errors: {}", error_msg)));
            }
        }

        Ok(response)
    }
}

/// Helper function to create GraphQL query request
pub fn create_query_request(query: impl Into<String>, variables: Option<Value>) -> GraphQLRequest {
    GraphQLRequest {
        query: Some(query.into()),
        mutation: None,
        variables,
        operation_name: None,
        timeout: None,
        headers: HashMap::new(),
    }
}

/// Helper function to create GraphQL mutation request
pub fn create_mutation_request(mutation: impl Into<String>, variables: Option<Value>) -> GraphQLRequest {
    GraphQLRequest {
        query: None,
        mutation: Some(mutation.into()),
        variables,
        operation_name: None,
        timeout: None,
        headers: HashMap::new(),
    }
}

/// Helper function to create GraphQL subscription request
pub fn create_subscription_request(
    subscription: impl Into<String>,
    variables: Option<Value>,
    operation_name: Option<String>,
) -> GraphQLRequest {
    GraphQLRequest {
        query: Some(subscription.into()),
        mutation: None,
        variables,
        operation_name,
        timeout: None,
        headers: HashMap::new(),
    }
}

/// GraphQL Client connection statistics
#[derive(Debug, Clone, Serialize)]
pub struct GraphQLConnectionStats {
    pub active_subscriptions: usize,
    pub server_uri: String,
    pub is_authenticated: bool,
    pub encryption_enabled: bool,
}

/// Implementation of missing methods for compatibility
impl GraphQLClient {
    /// Get connection statistics
    pub async fn get_stats(&self) -> GraphQLConnectionStats {
        GraphQLConnectionStats {
            active_subscriptions: self.subscriptions.read().await.len(),
            server_uri: self.server_uri.clone(),
            is_authenticated: self.auth_token.is_some(),
            encryption_enabled: self.encrypt,
        }
    }
    
    /// Health check method to verify server connectivity
    pub async fn health_check(&self) -> Result<bool> {
        let health_request = GraphQLRequest {
            query: Some("{ __typename }".to_string()),
            mutation: None,
            variables: None,
            operation_name: Some("HealthCheck".to_string()),
            timeout: Some(Duration::from_secs(5)),
            headers: HashMap::new(),
        };
        
        match self.query(health_request).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false)
        }
    }
    
    /// Unsubscribe from all active subscriptions (synchronous version)
    pub fn unsubscribe_all(&self) {
        // This is a simplified version for now - just a compatibility stub
        // Real implementations would use unsubscribe_all_async()
    }
}

#[cfg(test)]
mod cipher_hash_transport_tests {
    use super::*;

    fn body(query: &str, variables: Value) -> Value {
        json!({ "query": query, "variables": variables, "operationName": Value::Null })
    }

    fn client() -> GraphQLClient {
        GraphQLClient::new("http://127.0.0.1:1/graphql")
    }

    /// The auth bootstrap and introspection must stay plaintext — the key exchange
    /// cannot be encrypted with a key it has not established yet.
    #[test]
    fn bypass_set_is_not_encrypted() {
        let c = client();

        assert!(!c.should_encrypt(&body("query { __schema { types { name } } }", Value::Null)));
        assert!(!c.should_encrypt(&body("query ( $bundle: String ) { ContinuId ( bundle: $bundle ) { address } }", Value::Null)));
        assert!(!c.should_encrypt(&body("mutation { AccessToken { token } }", Value::Null)));
        assert!(!c.should_encrypt(&body(
            "mutation ( $molecule: MoleculeInput! ) { ProposeMolecule ( molecule: $molecule ) { status } }",
            json!({ "molecule": { "atoms": [ { "isotope": "U" } ] } }),
        )));
    }

    /// Everything else — including a ProposeMolecule that is NOT the U-isotope auth
    /// bootstrap — goes through the encrypted envelope.
    #[test]
    fn ordinary_operations_are_encrypted() {
        let c = client();

        assert!(c.should_encrypt(&body("query { Balance { address } }", Value::Null)));
        assert!(c.should_encrypt(&body(
            "mutation ( $molecule: MoleculeInput! ) { ProposeMolecule ( molecule: $molecule ) { status } }",
            json!({ "molecule": { "atoms": [ { "isotope": "M" } ] } }),
        )));
    }

    #[test]
    fn parse_operation_reads_type_and_root_field() {
        assert_eq!(parse_operation("query { Balance { address } }"), ("query".into(), "Balance".into()));
        assert_eq!(
            parse_operation("mutation ( $m: MoleculeInput! ) { ProposeMolecule ( molecule: $m ) { status } }"),
            ("mutation".into(), "ProposeMolecule".into())
        );
        // Anonymous selection sets default to `query`.
        assert_eq!(parse_operation("{ ContinuId { address } }"), ("query".into(), "ContinuId".into()));
    }

    /// Encryption enabled with no transport key must FAIL CLOSED rather than
    /// silently falling back to plaintext.
    #[tokio::test]
    async fn encryption_without_a_transport_key_fails_closed() {
        let mut c = client();
        c.set_encryption(true);

        let err = c
            .query(create_query_request("query { Balance { address } }", None))
            .await
            .expect_err("an encrypted request with no key must not be sent");
        assert!(
            err.to_string().contains("CipherHash: encryption is enabled but no ML-KEM transport key"),
            "expected the fail-closed error, got: {err}"
        );
    }

    /// A bypassed operation is NOT blocked by the missing-key check — it is allowed
    /// to reach the network (and fails there, against an unroutable URI).
    #[tokio::test]
    async fn bypassed_operation_is_not_blocked_by_the_key_check() {
        let mut c = client();
        c.set_encryption(true);

        let err = c
            .query(create_query_request("query { __schema { types { name } } }", None))
            .await
            .expect_err("an unroutable URI must fail at the network layer");
        assert!(
            !err.to_string().contains("CipherHash:"),
            "introspection must bypass the CipherHash key check, got: {err}"
        );
    }
}