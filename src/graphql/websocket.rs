//! WebSocket subscription management for GraphQL
//!
//! This module provides advanced WebSocket functionality for GraphQL subscriptions,
//! including connection pooling, auto-reconnection, and subscription lifecycle management.

use crate::error::{KnishIOError, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tracing::{debug, error, info, warn};
use tungstenite::Utf8Bytes;
use uuid::Uuid;

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// WebSocket subscription manager for handling multiple GraphQL subscriptions
#[derive(Clone)]
pub struct WebSocketManager {
    socket_uri: String,
    auth_token: Option<String>,
    app_key: String,
    state: Arc<RwLock<ConnectionState>>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
    connection_sender: Option<mpsc::UnboundedSender<WebSocketCommand>>,
    reconnect_config: ReconnectConfig,
    debug: bool,
}

/// Configuration for WebSocket reconnection behavior
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub connection_timeout: Duration,
    pub keep_alive_interval: Duration,
}

/// Information about an active subscription
#[derive(Debug, Clone)]
struct SubscriptionInfo {
    id: String,
    query: String,
    variables: Option<Value>,
    operation_name: Option<String>,
    callback_sender: mpsc::UnboundedSender<crate::GraphQLResponse>,
}

/// Commands for controlling the WebSocket connection
#[derive(Debug)]
enum WebSocketCommand {
    Subscribe {
        id: String,
        query: String,
        variables: Option<Value>,
        operation_name: Option<String>,
        callback_sender: mpsc::UnboundedSender<crate::GraphQLResponse>,
    },
    Unsubscribe {
        id: String,
    },
    Disconnect,
    Reconnect,
}

/// WebSocket message types following GraphQL WebSocket protocol
#[derive(Debug, Clone)]
enum GraphQLWsMessage {
    ConnectionInit { payload: Option<Value> },
    ConnectionAck,
    Start { id: String, payload: Value },
    Data { id: String, payload: Value },
    Error { id: String, payload: Value },
    Complete { id: String },
    Stop { id: String },
    ConnectionTerminate,
    KeepAlive,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        ReconnectConfig {
            max_attempts: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            connection_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
        }
    }
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(
        socket_uri: String,
        auth_token: Option<String>,
        app_key: String,
        reconnect_config: ReconnectConfig,
        debug: bool,
    ) -> Self {
        WebSocketManager {
            socket_uri,
            auth_token,
            app_key,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connection_sender: None,
            reconnect_config,
            debug,
        }
    }
    
    /// Start the WebSocket connection manager
    pub async fn start(&mut self) -> Result<()> {
        if self.connection_sender.is_some() {
            return Ok(()); // Already started
        }
        
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        self.connection_sender = Some(command_sender);
        
        let socket_uri = self.socket_uri.clone();
        let auth_token = self.auth_token.clone();
        let app_key = self.app_key.clone();
        let state = self.state.clone();
        let subscriptions = self.subscriptions.clone();
        let reconnect_config = self.reconnect_config.clone();
        let debug = self.debug;
        
        tokio::spawn(async move {
            Self::connection_loop(
                socket_uri,
                auth_token,
                app_key,
                state,
                subscriptions,
                command_receiver,
                reconnect_config,
                debug,
            ).await;
        });
        
        Ok(())
    }
    
    /// Subscribe to a GraphQL subscription
    pub async fn subscribe(
        &mut self,
        query: String,
        variables: Option<Value>,
        operation_name: Option<String>,
    ) -> Result<mpsc::UnboundedReceiver<crate::GraphQLResponse>> {
        self.start().await?;
        
        let id = Uuid::new_v4().to_string();
        let (callback_sender, callback_receiver) = mpsc::unbounded_channel();
        
        if let Some(ref sender) = self.connection_sender {
            sender.send(WebSocketCommand::Subscribe {
                id,
                query,
                variables,
                operation_name,
                callback_sender,
            }).map_err(|_| KnishIOError::WebSocketError("Failed to send subscribe command".into()))?;
        }
        
        Ok(callback_receiver)
    }
    
    /// Unsubscribe from a specific subscription
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<()> {
        if let Some(ref sender) = self.connection_sender {
            sender.send(WebSocketCommand::Unsubscribe {
                id: subscription_id.to_string(),
            }).map_err(|_| KnishIOError::WebSocketError("Failed to send unsubscribe command".into()))?;
        }
        Ok(())
    }
    
    /// Disconnect and cleanup all subscriptions
    pub async fn disconnect(&mut self) {
        if let Some(ref sender) = self.connection_sender {
            let _ = sender.send(WebSocketCommand::Disconnect);
        }
        self.connection_sender = None;
    }
    
    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionState {
        *self.state.read().await
    }
    
    /// Get number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }
    
    /// Force reconnection
    pub async fn reconnect(&self) -> Result<()> {
        if let Some(ref sender) = self.connection_sender {
            sender.send(WebSocketCommand::Reconnect)
                .map_err(|_| KnishIOError::WebSocketError("Failed to send reconnect command".into()))?;
        }
        Ok(())
    }
    
    /// Main connection loop that handles WebSocket lifecycle
    async fn connection_loop(
        socket_uri: String,
        auth_token: Option<String>,
        app_key: String,
        state: Arc<RwLock<ConnectionState>>,
        subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
        mut command_receiver: mpsc::UnboundedReceiver<WebSocketCommand>,
        reconnect_config: ReconnectConfig,
        debug: bool,
    ) {
        // Connection loop variables
        let mut reconnect_attempts = 0;
        
        loop {
            *state.write().await = ConnectionState::Connecting;
            
            match Self::establish_connection(
                &socket_uri,
                &auth_token,
                &app_key,
                &state,
                &subscriptions,
                &mut command_receiver,
                &reconnect_config,
                debug,
            ).await {
                Ok(_) => {
                    reconnect_attempts = 0;
                    if debug {
                        info!("WebSocket connection completed successfully");
                    }
                }
                Err(err) => {
                    reconnect_attempts += 1;
                    *state.write().await = ConnectionState::Failed;
                    
                    if debug {
                        error!("WebSocket connection failed (attempt {}): {}", reconnect_attempts, err);
                    }
                    
                    if reconnect_attempts >= reconnect_config.max_attempts {
                        if debug {
                            error!("Max reconnection attempts reached, giving up");
                        }
                        break;
                    }
                    
                    // Calculate delay with exponential backoff
                    let delay = std::cmp::min(
                        Duration::from_millis(
                            (reconnect_config.initial_delay.as_millis() as f64 * 
                             reconnect_config.backoff_multiplier.powi((reconnect_attempts - 1) as i32)) as u64
                        ),
                        reconnect_config.max_delay,
                    );
                    
                    if debug {
                        info!("Reconnecting in {:?}", delay);
                    }
                    
                    *state.write().await = ConnectionState::Reconnecting;
                    sleep(delay).await;
                }
            }
            
            // Check if we should continue or if disconnect was requested
            if let Ok(command) = command_receiver.try_recv() {
                if matches!(command, WebSocketCommand::Disconnect) {
                    break;
                }
                // Put the command back for processing
                // Note: This is a simplification; in practice you'd want a better way to handle this
            }
        }
        
        *state.write().await = ConnectionState::Disconnected;
        
        // Cleanup all subscriptions
        let mut subs = subscriptions.write().await;
        for (_, sub_info) in subs.drain() {
            // Send a final error to subscribers
            let error_response = crate::GraphQLResponse {
                data: None,
                errors: Some(vec![crate::GraphQLError {
                    message: "WebSocket connection closed".to_string(),
                    locations: None,
                    path: None,
                    extensions: None,
                }]),
                extensions: None,
            };
            let _ = sub_info.callback_sender.send(error_response);
        }
    }
    
    /// Establish and manage a single WebSocket connection
    async fn establish_connection(
        socket_uri: &str,
        auth_token: &Option<String>,
        app_key: &str,
        state: &Arc<RwLock<ConnectionState>>,
        subscriptions: &Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
        command_receiver: &mut mpsc::UnboundedReceiver<WebSocketCommand>,
        reconnect_config: &ReconnectConfig,
        debug: bool,
    ) -> Result<()> {
        // Connect to WebSocket
        let ws_stream = timeout(
            reconnect_config.connection_timeout,
            connect_async(socket_uri)
        )
        .await
        .map_err(|_| KnishIOError::WebSocketError("Connection timeout".into()))?
        .map_err(|e| KnishIOError::WebSocketError(format!("Connection failed: {}", e)))?
        .0;
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Send connection init
        let init_msg = GraphQLWsMessage::ConnectionInit {
            payload: Some(json!({
                "authToken": auth_token,
                "appKey": app_key
            }))
        };
        
        Self::send_ws_message(&mut ws_sender, &init_msg).await?;
        
        // Wait for connection_ack
        let ack_timeout = Duration::from_secs(10);
        let ack_result = timeout(ack_timeout, ws_receiver.next()).await;
        
        match ack_result {
            Ok(Some(Ok(Message::Text(text)))) => {
                if let Ok(msg) = Self::parse_ws_message(&text) {
                    if !matches!(msg, GraphQLWsMessage::ConnectionAck) {
                        return Err(KnishIOError::WebSocketError("Expected connection_ack".into()));
                    }
                } else {
                    return Err(KnishIOError::WebSocketError("Invalid connection_ack message".into()));
                }
            }
            _ => return Err(KnishIOError::WebSocketError("Failed to receive connection_ack".into())),
        }
        
        *state.write().await = ConnectionState::Connected;
        
        if debug {
            info!("WebSocket connected successfully");
        }
        
        // Resubscribe to existing subscriptions
        let current_subs: Vec<_> = {
            let subs = subscriptions.read().await;
            subs.values().cloned().collect()
        };
        
        for sub in current_subs {
            let start_msg = GraphQLWsMessage::Start {
                id: sub.id.clone(),
                payload: json!({
                    "query": sub.query,
                    "variables": sub.variables,
                    "operationName": sub.operation_name
                })
            };
            Self::send_ws_message(&mut ws_sender, &start_msg).await?;
        }
        
        // Set up keep-alive
        let mut keep_alive_interval = interval(reconnect_config.keep_alive_interval);
        
        // Main message loop
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                ws_msg = ws_receiver.next() => {
                    match ws_msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = Self::handle_ws_message(
                                &text,
                                subscriptions,
                                debug
                            ).await {
                                if debug {
                                    warn!("Error handling WebSocket message: {}", e);
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            if debug {
                                info!("WebSocket connection closed by server");
                            }
                            return Err(KnishIOError::WebSocketError("Connection closed by server".into()));
                        }
                        Some(Err(e)) => {
                            return Err(KnishIOError::WebSocketError(format!("Stream error: {}", e)));
                        }
                        None => {
                            return Err(KnishIOError::WebSocketError("Stream ended".into()));
                        }
                        _ => {} // Ignore other message types
                    }
                }
                
                // Handle commands from the client
                command = command_receiver.recv() => {
                    match command {
                        Some(WebSocketCommand::Subscribe { id, query, variables, operation_name, callback_sender }) => {
                            let sub_info = SubscriptionInfo {
                                id: id.clone(),
                                query: query.clone(),
                                variables: variables.clone(),
                                operation_name: operation_name.clone(),
                                callback_sender,
                            };
                            
                            subscriptions.write().await.insert(id.clone(), sub_info);
                            
                            let start_msg = GraphQLWsMessage::Start {
                                id,
                                payload: json!({
                                    "query": query,
                                    "variables": variables,
                                    "operationName": operation_name
                                })
                            };
                            
                            if let Err(e) = Self::send_ws_message(&mut ws_sender, &start_msg).await {
                                if debug {
                                    error!("Failed to send subscription start: {}", e);
                                }
                                return Err(e);
                            }
                        }
                        
                        Some(WebSocketCommand::Unsubscribe { id }) => {
                            subscriptions.write().await.remove(&id);
                            
                            let stop_msg = GraphQLWsMessage::Stop { id };
                            if let Err(e) = Self::send_ws_message(&mut ws_sender, &stop_msg).await {
                                if debug {
                                    error!("Failed to send subscription stop: {}", e);
                                }
                            }
                        }
                        
                        Some(WebSocketCommand::Disconnect) => {
                            if debug {
                                info!("Disconnect requested");
                            }
                            
                            let terminate_msg = GraphQLWsMessage::ConnectionTerminate;
                            let _ = Self::send_ws_message(&mut ws_sender, &terminate_msg).await;
                            return Ok(());
                        }
                        
                        Some(WebSocketCommand::Reconnect) => {
                            if debug {
                                info!("Reconnect requested");
                            }
                            return Err(KnishIOError::WebSocketError("Reconnect requested".into()));
                        }
                        
                        None => {
                            if debug {
                                info!("Command channel closed");
                            }
                            return Err(KnishIOError::WebSocketError("Command channel closed".into()));
                        }
                    }
                }
                
                // Send keep-alive messages
                _ = keep_alive_interval.tick() => {
                    let ka_msg = GraphQLWsMessage::KeepAlive;
                    if let Err(e) = Self::send_ws_message(&mut ws_sender, &ka_msg).await {
                        if debug {
                            warn!("Failed to send keep-alive: {}", e);
                        }
                        return Err(e);
                    }
                }
            }
        }
    }
    
    /// Send a GraphQL WebSocket message
    async fn send_ws_message(
        sender: &mut futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Message>,
        message: &GraphQLWsMessage,
    ) -> Result<()> {
        let text = Self::serialize_ws_message(message)?;
        sender.send(Message::Text(Utf8Bytes::from(text)))
            .await
            .map_err(|e| KnishIOError::WebSocketError(format!("Failed to send message: {}", e)))
    }
    
    /// Handle incoming WebSocket message
    async fn handle_ws_message(
        text: &str,
        subscriptions: &Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
        debug: bool,
    ) -> Result<()> {
        let message = Self::parse_ws_message(text)?;
        
        match message {
            GraphQLWsMessage::Data { id, payload } => {
                let subs = subscriptions.read().await;
                if let Some(sub_info) = subs.get(&id) {
                    if let Ok(response) = serde_json::from_value::<crate::GraphQLResponse>(payload) {
                        if let Err(_) = sub_info.callback_sender.send(response) {
                            if debug {
                                warn!("Failed to send data to subscription {}: receiver dropped", id);
                            }
                        }
                    }
                }
            }
            
            GraphQLWsMessage::Error { id, payload } => {
                let subs = subscriptions.read().await;
                if let Some(sub_info) = subs.get(&id) {
                    let error_response = crate::GraphQLResponse {
                        data: None,
                        errors: Some(vec![crate::GraphQLError {
                            message: payload.as_str().unwrap_or("Subscription error").to_string(),
                            locations: None,
                            path: None,
                            extensions: None,
                        }]),
                        extensions: None,
                    };
                    let _ = sub_info.callback_sender.send(error_response);
                }
            }
            
            GraphQLWsMessage::Complete { id } => {
                if debug {
                    info!("Subscription {} completed by server", id);
                }
                // Remove the subscription but don't send error
                subscriptions.write().await.remove(&id);
            }
            
            GraphQLWsMessage::KeepAlive => {
                // Keep-alive received, no action needed
            }
            
            _ => {
                if debug {
                    debug!("Received unexpected WebSocket message type");
                }
            }
        }
        
        Ok(())
    }
    
    /// Parse a WebSocket message from text
    fn parse_ws_message(text: &str) -> Result<GraphQLWsMessage> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| KnishIOError::WebSocketError(format!("Failed to parse message: {}", e)))?;
        
        let msg_type = value.get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| KnishIOError::WebSocketError("Missing message type".into()))?;
        
        match msg_type {
            "connection_ack" => Ok(GraphQLWsMessage::ConnectionAck),
            "data" => {
                let id = value.get("id")
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| KnishIOError::WebSocketError("Missing subscription ID".into()))?;
                let payload = value.get("payload")
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(GraphQLWsMessage::Data { id: id.to_string(), payload })
            }
            "error" => {
                let id = value.get("id")
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| KnishIOError::WebSocketError("Missing subscription ID".into()))?;
                let payload = value.get("payload")
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(GraphQLWsMessage::Error { id: id.to_string(), payload })
            }
            "complete" => {
                let id = value.get("id")
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| KnishIOError::WebSocketError("Missing subscription ID".into()))?;
                Ok(GraphQLWsMessage::Complete { id: id.to_string() })
            }
            "ka" => Ok(GraphQLWsMessage::KeepAlive),
            _ => Err(KnishIOError::WebSocketError(format!("Unknown message type: {}", msg_type)))
        }
    }
    
    /// Serialize a WebSocket message to text
    fn serialize_ws_message(message: &GraphQLWsMessage) -> Result<String> {
        let value = match message {
            GraphQLWsMessage::ConnectionInit { payload } => json!({
                "type": "connection_init",
                "payload": payload
            }),
            GraphQLWsMessage::Start { id, payload } => json!({
                "type": "start",
                "id": id,
                "payload": payload
            }),
            GraphQLWsMessage::Stop { id } => json!({
                "type": "stop",
                "id": id
            }),
            GraphQLWsMessage::ConnectionTerminate => json!({
                "type": "connection_terminate"
            }),
            GraphQLWsMessage::KeepAlive => json!({
                "type": "ka"
            }),
            _ => return Err(KnishIOError::WebSocketError("Cannot serialize this message type".into())),
        };
        
        serde_json::to_string(&value)
            .map_err(|e| KnishIOError::WebSocketError(format!("Failed to serialize message: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reconnect_config_default() {
        let config = ReconnectConfig::default();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.backoff_multiplier, 2.0);
    }
    
    #[test]
    fn test_ws_message_serialization() {
        let init_msg = GraphQLWsMessage::ConnectionInit {
            payload: Some(json!({"authToken": "test"})),
        };
        
        let serialized = WebSocketManager::serialize_ws_message(&init_msg).unwrap();
        assert!(serialized.contains("connection_init"));
        assert!(serialized.contains("authToken"));
    }
    
    #[test]
    fn test_ws_message_parsing() {
        let text = r#"{"type":"connection_ack"}"#;
        let parsed = WebSocketManager::parse_ws_message(text).unwrap();
        assert!(matches!(parsed, GraphQLWsMessage::ConnectionAck));
        
        let data_text = r#"{"type":"data","id":"sub1","payload":{"data":{}}}"#;
        let parsed_data = WebSocketManager::parse_ws_message(data_text).unwrap();
        if let GraphQLWsMessage::Data { id, .. } = parsed_data {
            assert_eq!(id, "sub1");
        } else {
            panic!("Expected Data message");
        }
    }
    
    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let manager = WebSocketManager::new(
            "ws://localhost:8080/graphql".to_string(),
            Some("test-token".to_string()),
            "knishio".to_string(),
            ReconnectConfig::default(),
            true,
        );
        
        assert_eq!(manager.get_state().await, ConnectionState::Disconnected);
        assert_eq!(manager.subscription_count().await, 0);
    }
}