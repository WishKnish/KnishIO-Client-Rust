//! KnishIO client implementation
//!
//! This module provides the main client interface for interacting with
//! KnishIO distributed ledger nodes.

pub mod builder;

use crate::error::{KnishIOError, Result};
use crate::wallet::Wallet;
use crate::auth::AuthToken;
use crate::molecule::Molecule;
use crate::response::{Response};
use crate::graphql::{
    GraphQLClient, SocketConfig
};
use crate::subscribe::{
    SubscriptionManager, SubscriptionEvent, SubscriptionHandle, Subscribe,
    CreateMoleculeSubscribe, WalletStatusSubscribe, ActiveWalletSubscribe, ActiveSessionSubscribe
};
use crate::subscribe::simple_websocket::SimpleWebSocketClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rand;

/// Recipient type for request_tokens() method
///
/// Handles different ways to specify a token recipient:
/// - BundleHash: 64-char hex string representing a wallet bundle
/// - Secret: User secret to create a new wallet
/// - Wallet: Pre-existing wallet instance
/// - None: Request tokens for self (uses client's bundle)
#[derive(Debug, Clone)]
pub enum RecipientType {
    /// Bundle hash (64 hex chars)
    BundleHash(String),
    /// Secret to create wallet from
    Secret(String),
    /// Existing wallet
    Wallet(Wallet),
}

/// Main KnishIO client (equivalent to KnishIOClient.js)
/// 
/// Provides the primary interface for interacting with KnishIO distributed ledger nodes.
/// Supports molecule creation, wallet management, GraphQL queries, and real-time subscriptions.
/// 
/// # Examples
/// 
/// ```rust
/// use knishio_client::KnishIOClient;
/// 
/// let client = KnishIOClient::new(vec!["https://api.knish.io".to_string()], None);
/// // client.set_secret("your-secret-here");
/// ```
pub struct KnishIOClient {
    /// List of KnishIO node URIs for GraphQL communication
    uris: Vec<String>,
    /// Current URI index for round-robin load balancing
    current_uri_index: usize,
    /// Optional cell slug for targeting specific sub-ledgers
    cell_slug: Option<String>,
    /// User secret for cryptographic operations and wallet generation
    secret: Option<String>,
    /// Bundle hash (64-character user identifier derived from secret)
    bundle: Option<String>,
    
    /// Current authentication token for server requests
    auth_token: Option<AuthToken>,
    /// Map of authentication tokens by context
    auth_token_objects: HashMap<String, AuthToken>,
    /// Flag indicating if authentication is in progress
    auth_in_process: bool,
    
    /// Server SDK version for compatibility checks
    server_sdk_version: u32,
    /// Whether to encrypt communications (ML-KEM quantum encryption)
    encrypt: bool,
    /// Whether to enable debug logging
    logging: bool,
    
    /// GraphQL client for node communication
    client: Option<GraphQLClient>,
    /// WebSocket configuration for real-time subscriptions
    socket_config: Option<SocketConfig>,
    /// WebSocket client for GraphQL subscriptions
    #[allow(dead_code)]
    websocket_client: Option<SimpleWebSocketClient>,
    /// Subscription manager for handling real-time subscriptions
    subscription_manager: Option<Arc<SubscriptionManager>>,
    
    /// Last remainder wallet from molecule operations
    remainder_wallet: Option<Wallet>,
    /// Last molecule query identifier for tracking state
    last_molecule_query: Option<String>,
    
    /// Abort controllers for cancelling in-flight requests
    abort_controllers: Arc<Mutex<HashMap<String, bool>>>,
}

impl KnishIOClient {
    /// Create a new KnishIO client (equivalent to constructor)
    pub fn new(
        uri: impl Into<UriParam>,
        cell_slug: Option<String>,
        socket: Option<SocketConfig>,
        client: Option<GraphQLClient>,
        server_sdk_version: Option<u32>,
        logging: Option<bool>,
    ) -> Self {
        let mut client_instance = KnishIOClient {
            uris: Vec::new(),
            current_uri_index: 0,
            cell_slug: None,
            secret: None,
            bundle: None,
            auth_token: None,
            auth_token_objects: HashMap::new(),
            auth_in_process: false,
            server_sdk_version: server_sdk_version.unwrap_or(3),
            encrypt: false,
            logging: logging.unwrap_or(false),
            client: None,
            socket_config: socket.clone(),
            websocket_client: None,
            subscription_manager: None,
            remainder_wallet: None,
            last_molecule_query: None,
            abort_controllers: Arc::new(Mutex::new(HashMap::new())),
        };

        client_instance.initialize(uri, cell_slug, socket, client, server_sdk_version, logging);
        client_instance
    }

    /// Initialize the client with given parameters
    pub fn initialize(
        &mut self,
        uri: impl Into<UriParam>,
        cell_slug: Option<String>,
        _socket: Option<SocketConfig>,
        client: Option<GraphQLClient>,
        server_sdk_version: Option<u32>,
        logging: Option<bool>,
    ) {
        self.reset();

        self.logging = logging.unwrap_or(false);
        self.auth_token_objects.clear();
        self.auth_in_process = false;
        self.abort_controllers = Arc::new(Mutex::new(HashMap::new()));

        if let Err(e) = self.set_uri(uri) {
            self.log("error", &format!("Failed to set URI: {}", e));
        }

        if let Some(cell) = cell_slug {
            self.set_cell_slug(cell);
        }

        for uri in &self.uris {
            // Create an empty AuthToken for now
            let auth_token = AuthToken::new(String::new(), None, None, None);
            self.auth_token_objects.insert(uri.clone(), auth_token);
        }

        self.log("info", &format!("KnishIOClient::initialize() - Initializing new Knish.IO client session for SDK version {}...", self.server_sdk_version));

        if let Some(client) = client {
            self.client = Some(client.clone());
            
            // Initialize subscription manager with the GraphQL client
            self.subscription_manager = Some(Arc::new(SubscriptionManager::new(Arc::new(client))));
        } else {
            let uri = self.get_random_uri();
            let new_client = GraphQLClient::new(uri);
            self.client = Some(new_client.clone());
            // Initialize subscription manager with the new GraphQL client
            self.subscription_manager = Some(Arc::new(SubscriptionManager::new(Arc::new(new_client))));
        }

        self.server_sdk_version = server_sdk_version.unwrap_or(3);
    }

    /// Get the subscription manager for real-time subscriptions
    pub fn get_subscription_manager(&self) -> Result<Arc<SubscriptionManager>> {
        self.subscription_manager.as_ref()
            .cloned()
            .ok_or_else(|| KnishIOError::custom("Subscription manager not initialized"))
    }

    /// Subscribe to CreateMolecule events (equivalent to subscribeCreateMolecule in JS)
    pub async fn subscribe_create_molecule<F>(&self, bundle: Option<String>, callback: F) -> Result<SubscriptionHandle>
    where
        F: Fn(SubscriptionEvent) + Send + Sync + 'static,
    {
        let _manager = self.get_subscription_manager()?;
        let graphql_client = self.client.as_ref()
            .ok_or_else(|| KnishIOError::custom("GraphQL client not initialized"))?;
        
        let subscription = CreateMoleculeSubscribe::new(Arc::new(graphql_client.clone()));
        
        let bundle = bundle.unwrap_or_else(|| self.get_bundle().unwrap_or_default().to_string());
        let variables = json!({
            "bundle": bundle
        });
        
        // Convert callback to Box<dyn Fn(Value)> for JavaScript compatibility
        let boxed_callback = Box::new(move |data: Value| {
            let event = SubscriptionEvent::new("CreateMolecule".to_string(), data);
            callback(event);
        });
        
        subscription.execute(variables, boxed_callback).await
    }

    /// Subscribe to WalletStatus events (equivalent to subscribeWalletStatus in JS)
    pub async fn subscribe_wallet_status<F>(&self, bundle: Option<String>, token: String, callback: F) -> Result<SubscriptionHandle>
    where
        F: Fn(SubscriptionEvent) + Send + Sync + 'static,
    {
        if token.is_empty() {
            return Err(KnishIOError::custom("Token parameter is required for wallet status subscription"));
        }

        let _manager = self.get_subscription_manager()?;
        let graphql_client = self.client.as_ref()
            .ok_or_else(|| KnishIOError::custom("GraphQL client not initialized"))?;
        
        let subscription = WalletStatusSubscribe::new(Arc::new(graphql_client.clone()));
        
        let bundle = bundle.unwrap_or_else(|| self.get_bundle().unwrap_or_default().to_string());
        let variables = json!({
            "bundle": bundle,
            "token": token
        });
        
        // Convert callback to Box<dyn Fn(Value)> for JavaScript compatibility
        let boxed_callback = Box::new(move |data: Value| {
            let event = SubscriptionEvent::new("WalletStatus".to_string(), data);
            callback(event);
        });
        
        subscription.execute(variables, boxed_callback).await
    }

    /// Subscribe to ActiveWallet events (equivalent to subscribeActiveWallet in JS)
    pub async fn subscribe_active_wallet<F>(&self, bundle: Option<String>, callback: F) -> Result<SubscriptionHandle>
    where
        F: Fn(SubscriptionEvent) + Send + Sync + 'static,
    {
        let _manager = self.get_subscription_manager()?;
        let graphql_client = self.client.as_ref()
            .ok_or_else(|| KnishIOError::custom("GraphQL client not initialized"))?;
        
        let subscription = ActiveWalletSubscribe::new(Arc::new(graphql_client.clone()));
        
        let bundle = bundle.unwrap_or_else(|| self.get_bundle().unwrap_or_default().to_string());
        let variables = json!({
            "bundle": bundle
        });
        
        // Convert callback to Box<dyn Fn(Value)> for JavaScript compatibility
        let boxed_callback = Box::new(move |data: Value| {
            let event = SubscriptionEvent::new("ActiveWallet".to_string(), data);
            callback(event);
        });
        
        subscription.execute(variables, boxed_callback).await
    }

    /// Subscribe to ActiveSession events (equivalent to subscribeActiveSession in JS)
    pub async fn subscribe_active_session<F>(&self, meta_type: String, meta_id: String, callback: F) -> Result<SubscriptionHandle>
    where
        F: Fn(SubscriptionEvent) + Send + Sync + 'static,
    {
        let _manager = self.get_subscription_manager()?;
        let graphql_client = self.client.as_ref()
            .ok_or_else(|| KnishIOError::custom("GraphQL client not initialized"))?;
        
        let subscription = ActiveSessionSubscribe::new(Arc::new(graphql_client.clone()));
        
        let variables = json!({
            "metaType": meta_type,
            "metaId": meta_id
        });
        
        // Convert callback to Box<dyn Fn(Value)> for JavaScript compatibility
        let boxed_callback = Box::new(move |data: Value| {
            let event = SubscriptionEvent::new("ActiveSession".to_string(), data);
            callback(event);
        });
        
        subscription.execute(variables, boxed_callback).await
    }

    /// Create a Query instance of the specified type (equivalent to createQuery in TS)
    ///
    /// Matches TS createQuery<T extends Query>(QueryClass) at lines 651-653
    ///
    /// # Type Parameters
    /// - `T`: Query type that implements Default
    ///
    /// # Returns
    /// New instance of the specified query type
    pub fn create_query<T>(&self) -> T
    where
        T: Default + crate::query::Query,
    {
        T::default()
    }

    /// Create a Subscribe instance of the specified type (equivalent to createSubscribe in JS)
    pub fn create_subscribe<T>(&self, graphql_client: GraphQLClient) -> Result<T>
    where
        T: From<GraphQLClient>,
    {
        Ok(T::from(graphql_client))
    }

    /// Connect to WebSocket for subscriptions
    pub async fn connect_subscription_websocket(&self) -> Result<()> {
        if let Some(manager) = &self.subscription_manager {
            manager.connect().await
        } else {
            Err(KnishIOError::custom("Subscription manager not initialized"))
        }
    }

    /// Disconnect from subscription WebSocket
    pub async fn disconnect_subscription_websocket(&self) -> Result<()> {
        if let Some(manager) = &self.subscription_manager {
            manager.disconnect().await
        } else {
            Ok(())
        }
    }

    /// Check if subscription WebSocket is connected
    pub async fn is_subscription_websocket_connected(&self) -> bool {
        match &self.subscription_manager {
            Some(manager) => manager.is_connected().await,
            None => false,
        }
    }

    /// Stop all active subscriptions
    pub async fn stop_all_subscriptions(&self) -> Result<()> {
        if let Some(manager) = &self.subscription_manager {
            manager.stop_all().await
        } else {
            Ok(())
        }
    }

    /// Get active subscription count
    pub async fn active_subscription_count(&self) -> usize {
        match &self.subscription_manager {
            Some(manager) => manager.active_count().await,
            None => 0,
        }
    }

    /// List all active subscription IDs
    pub async fn list_active_subscriptions(&self) -> Vec<String> {
        match &self.subscription_manager {
            Some(manager) => manager.list_subscriptions().await,
            None => Vec::new(),
        }
    }

    /// Get a specific subscription by ID
    pub async fn get_subscription_by_id(&self, id: &str) -> Option<String> {
        if let Some(manager) = &self.subscription_manager {
            manager.get_subscription(id).await
        } else {
            None
        }
    }

    /// Unsubscribe from a specific subscription by operation name (equivalent to unsubscribe in JS)
    ///
    /// # Arguments
    ///
    /// * `operation_name` - The name of the subscription operation to stop
    pub async fn unsubscribe(&self, operation_name: &str) {
        if let Some(manager) = &self.subscription_manager {
            manager.unsubscribe(operation_name).await;
        }
    }

    /// Unsubscribe from all active subscriptions (equivalent to unsubscribeAll in JS)
    ///
    /// This is an alias for `stop_all_subscriptions()` for JS SDK compatibility.
    pub async fn unsubscribe_all(&self) {
        if let Some(manager) = &self.subscription_manager {
            manager.unsubscribe_all().await;
        }
    }

    /// Reset the client state
    pub fn reset(&mut self) {
        self.secret = None;
        self.bundle = None;
        self.auth_token = None;
        self.remainder_wallet = None;
        self.last_molecule_query = None;
    }

    /// De-initialize the client session (equivalent to deinitialize in JS)
    ///
    /// Clears the Knish.IO client session so that a new session can replace it.
    /// This is an alias for `reset()` with semantic clarity for session cleanup.
    pub fn deinitialize(&mut self) {
        self.log("info", "KnishIOClient::deinitialize() - Clearing the Knish.IO client session...");
        self.reset();
    }

    /// Set the URI(s) for the client
    pub fn set_uri(&mut self, uri: impl Into<UriParam>) -> Result<()> {
        let param: UriParam = uri.into();
        match param {
            UriParam::Single(uri) => self.uris = vec![uri],
            UriParam::Multiple(uris) => self.uris = uris,
        }
        
        // Update the client's URI if it exists
        if let Some(ref mut _client) = self.client {
            if !self.uris.is_empty() {
                // Update client with a random URI from the list
                let uri = self.get_random_uri();
                // Note: GraphQL client doesn't have a set_uri method, so we'd need to recreate it
                // For now, we'll just log the change
                self.log("info", &format!("URI updated to: {}", uri));
            }
        }
        Ok(())
    }

    /// Set the cell slug
    pub fn set_cell_slug(&mut self, cell_slug: impl Into<String>) {
        self.cell_slug = Some(cell_slug.into());
    }

    /// Get a random URI from the list
    pub fn get_random_uri(&self) -> String {
        if self.uris.is_empty() {
            return String::new();
        }
        let index = rand::random_range(0..self.uris.len());
        self.uris[index].clone()
    }

    /// Check if the client has a secret
    pub fn has_secret(&self) -> bool {
        self.secret.is_some()
    }

    /// Check if the client has a bundle
    pub fn has_bundle(&self) -> bool {
        self.bundle.is_some()
    }

    /// Get the bundle hash
    pub fn get_bundle(&self) -> Option<&str> {
        self.bundle.as_deref()
    }

    /// Get the stored secret (equivalent to getSecret in JS)
    ///
    /// Returns the user's secret for cryptographic operations.
    ///
    /// # Returns
    ///
    /// Result containing reference to the secret
    ///
    /// # Errors
    ///
    /// Returns `Unauthenticated` error if no secret is set
    pub fn get_secret(&self) -> Result<&str> {
        self.secret.as_deref()
            .ok_or(KnishIOError::Unauthenticated)
    }

    /// Get the cell slug (equivalent to getCellSlug in JS)
    ///
    /// Returns the currently defined cell identifier for this session.
    ///
    /// # Returns
    ///
    /// Option containing reference to the cell slug if set
    pub fn get_cell_slug(&self) -> Option<&str> {
        self.cell_slug.as_deref()
    }

    /// Convenience alias for get_cell_slug() (equivalent to cellSlug in JS)
    ///
    /// # Returns
    ///
    /// Option containing reference to the cell slug if set
    pub fn cell_slug(&self) -> Option<&str> {
        self.get_cell_slug()
    }

    /// Get the current active URI (equivalent to getUri in JS)
    ///
    /// Returns the URI currently being used by the GraphQL client.
    /// Falls back to cached current URI if client not initialized.
    ///
    /// # Returns
    ///
    /// Option containing the current URI string
    pub fn get_uri(&self) -> Option<String> {
        // Try to get from GraphQL client first (if available)
        if let Some(client) = &self.client {
            Some(client.get_uri().to_string())
        } else {
            // Fall back to cached current URI
            self.get_current_uri()
        }
    }

    /// Convenience alias for get_uri() (equivalent to uri in JS)
    ///
    /// # Returns
    ///
    /// Option containing the current URI string
    pub fn uri(&self) -> Option<String> {
        self.get_uri()
    }

    /// Hash a secret to produce a bundle hash (equivalent to hashSecret in JS)
    ///
    /// Computes the wallet bundle hash from a given secret using SHAKE256.
    /// This is primarily used internally by setSecret() but exposed for
    /// testing and external bundle generation.
    ///
    /// # Arguments
    ///
    /// * `secret` - The secret to hash
    ///
    /// # Returns
    ///
    /// 64-character hexadecimal bundle hash
    pub fn hash_secret(&self, secret: &str) -> String {
        use crate::crypto::generate_bundle_hash;
        self.log("info", "KnishIOClient::hash_secret() - Computing wallet bundle from secret...");
        generate_bundle_hash(secret)
    }

    // =================== Wallet Lifecycle Management ===================

    /// Get the current remainder wallet (equivalent to getRemainderWallet in JS)
    ///
    /// Returns the remainder wallet from the last molecule operation.
    /// This is critical for ContinuID relay race progression.
    ///
    /// # Returns
    ///
    /// Option containing reference to the remainder wallet if it exists
    pub fn get_remainder_wallet(&self) -> Option<&Wallet> {
        self.remainder_wallet.as_ref()
    }

    /// Get the source wallet for molecule operations (equivalent to getSourceWallet in JS)
    ///
    /// Queries ContinuID for the latest wallet position. If no ContinuID exists,
    /// creates a new wallet from the secret. This is the starting point for
    /// the ContinuID relay race.
    ///
    /// # Returns
    ///
    /// Result containing the source wallet ready for molecule operations
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No secret is set
    /// - ContinuID query fails
    /// - Wallet creation fails
    pub async fn get_source_wallet(&mut self) -> Result<Wallet> {
        // Query ContinuID for latest wallet
        let continu_id_result = self.query_continu_id(self.get_bundle()).await?;

        let mut source_wallet = if let Some(wallet) = continu_id_result {
            // ContinuID exists, use it as source
            wallet
        } else {
            // No ContinuID, create new wallet from secret
            let secret = self.secret.as_ref()
                .ok_or_else(|| KnishIOError::MissingSecret)?;

            Wallet::new(
                Some(secret.as_str()),
                None,  // bundle will be auto-generated from secret
                None,  // token defaults to "USER"
                None,  // address
                None,  // position will be auto-generated
                None,  // batch_id
                None,  // characters
            )?
        };

        // Generate wallet key if we have position
        if let Some(position) = &source_wallet.position {
            let secret = self.secret.as_ref()
                .ok_or_else(|| KnishIOError::MissingSecret)?;

            source_wallet.key = Some(Wallet::generate_key(
                secret,
                &source_wallet.token,
                position
            ));
        }

        Ok(source_wallet)
    }

    /// Create a new Molecule for transaction operations (equivalent to createMolecule in JS)
    ///
    /// This method instantiates a new Molecule with proper source and remainder wallets,
    /// implementing the ContinuID relay race pattern. If no source wallet is provided,
    /// it will attempt to use the last remainder wallet (for continuity) or create a new one.
    ///
    /// # Arguments
    ///
    /// * `secret` - Optional secret (defaults to client secret)
    /// * `bundle` - Optional bundle hash (defaults to client bundle)
    /// * `source_wallet` - Optional source wallet (will be determined if not provided)
    /// * `remainder_wallet` - Optional remainder wallet (will be created if not provided)
    ///
    /// # Returns
    ///
    /// Result containing the newly created Molecule ready for transaction operations
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No secret is available
    /// - Source wallet determination fails
    /// - Remainder wallet creation fails
    pub async fn create_molecule(
        &mut self,
        secret: Option<String>,
        bundle: Option<String>,
        source_wallet: Option<Wallet>,
        remainder_wallet: Option<Wallet>,
    ) -> Result<Molecule> {
        self.log("info", "KnishIOClient::create_molecule() - Creating a new molecule...");

        // Use provided or get stored secret/bundle
        let secret = secret.or_else(|| self.secret.clone())
            .ok_or_else(|| KnishIOError::MissingSecret)?;
        let bundle = bundle.or_else(|| self.bundle.clone());

        // Determine source wallet
        let source_wallet = if let Some(wallet) = source_wallet {
            // Source wallet provided
            wallet
        } else if let Some(remainder) = &self.remainder_wallet {
            // Try to use last remainder wallet (ContinuID relay race)
            // Check conditions: token === 'USER' and last molecule was successful
            if remainder.token == "USER" && self.last_molecule_query.is_some() {
                // Use remainder wallet as source for continuity
                remainder.clone()
            } else {
                // Can't use remainder, get source wallet
                self.get_source_wallet().await?
            }
        } else {
            // No remainder wallet exists, get source wallet
            self.get_source_wallet().await?
        };

        // Create remainder wallet for next transaction
        let remainder = if let Some(wallet) = remainder_wallet {
            wallet
        } else {
            // Create new remainder wallet
            Wallet::new(
                Some(&secret),
                bundle.as_deref(),
                Some("USER"),  // Always USER token for remainder
                None,  // address will be generated
                None,  // position will be generated
                source_wallet.batch_id.as_deref(),
                source_wallet.characters.as_deref(),
            )?
        };

        // Store remainder wallet for next molecule
        self.remainder_wallet = Some(remainder.clone());

        // Create and configure molecule
        let mut molecule = Molecule::new();
        molecule.secret = Some(secret);
        molecule.source_wallet = Some(source_wallet);
        molecule.remainder_wallet = Some(remainder);
        molecule.cell_slug = self.cell_slug.clone();
        molecule.version = Some(self.server_sdk_version.to_string());
        molecule.bundle = bundle;

        Ok(molecule)
    }

    /// Submit a pre-built, pre-signed molecule directly to the ledger.
    ///
    /// Unlike higher-level methods (create_token, transfer_token, etc.) which build
    /// and sign molecules internally, this method accepts a molecule that has already
    /// been constructed and signed by the caller. This is useful for:
    /// - Validation testing (submitting intentionally corrupted molecules)
    /// - Advanced molecule composition patterns
    /// - Custom isotope operations
    ///
    /// # Arguments
    ///
    /// * `molecule` - A pre-built and pre-signed Molecule
    ///
    /// # Returns
    ///
    /// Result containing the server response
    ///
    /// # Errors
    ///
    /// Returns error if the client is not initialized or the server rejects the molecule
    pub async fn propose_molecule(&mut self, molecule: Molecule) -> Result<Box<dyn Response>> {
        use crate::mutation::propose_molecule::MutationProposeMolecule;
        use crate::mutation::Mutation;

        let mutation = MutationProposeMolecule::from_molecule(molecule);

        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Log a message if logging is enabled
    pub fn log(&self, level: &str, message: &str) {
        if self.logging {
            match level {
                "info" => println!("[INFO] {}", message),
                "warn" => println!("[WARN] {}", message),
                "error" => eprintln!("[ERROR] {}", message),
                _ => println!("[LOG] {}", message),
            }
        }
    }

    // =================== Authentication Token Lifecycle Management ===================
    
    /// Request authorization from the server (equivalent to requestAuthorization in JS)
    ///
    /// # Arguments
    ///
    /// * `meta` - Optional metadata for the authorization request
    ///
    /// # Returns
    ///
    /// Result indicating success of authorization request
    pub async fn request_authorization(&mut self, meta: Option<HashMap<String, serde_json::Value>>) -> Result<bool> {
        use crate::mutation::request_authorization::MutationRequestAuthorization;
        use crate::mutation::Mutation;
        use crate::types::MetaItem;

        // Check if we have a secret (before setting flag — no cleanup needed on this error)
        let secret = self.secret.clone()
            .ok_or(KnishIOError::MissingSecret)?;

        // Set authentication in process — must be reset on ALL exit paths below
        self.auth_in_process = true;

        // Inner block captures Result so we can always reset the flag
        let result: Result<bool> = async {
            // Create AUTH wallet from secret
            let auth_wallet = Wallet::new(
                Some(&secret),
                None,
                Some("AUTH"),
                None,
                None,
                None,
                None,
            )?;

            // Create molecule with secret and source wallet
            let mut molecule = Molecule::new();
            molecule.secret = Some(secret.clone());
            molecule.source_wallet = Some(auth_wallet.clone());

            // Convert meta HashMap to Vec<MetaItem> if provided
            let meta_items: Vec<MetaItem> = meta.unwrap_or_default()
                .iter()
                .map(|(k, v)| MetaItem::new(k, &v.to_string()))
                .collect();

            // Initialize authorization on molecule
            molecule.init_authorization(meta_items)?;

            // Sign the molecule
            molecule.sign(None, false, true)?;

            // Check molecule integrity
            molecule.check(None)?;

            // Create mutation (need GraphQL client)
            if let Some(ref client) = self.client {
                let mutation = MutationRequestAuthorization::from_molecule(molecule);
                let response = mutation.execute(client, None, None).await?;
                let success = response.success();

                if success {
                    self.log("info", "Authorization request completed successfully");
                } else {
                    self.log("error", &format!("Authorization request failed: {:?}", response.reason()));
                }

                Ok(success)
            } else {
                Err(KnishIOError::NoClient)
            }
        }.await;

        // Always reset flag, regardless of success or failure
        self.auth_in_process = false;
        result
    }
    
    /// Authenticate with the server using credentials (equivalent to authenticate in JS)
    ///
    /// # Arguments
    ///
    /// * `meta` - Authentication metadata (e.g., username, password, etc.)
    ///
    /// # Returns
    ///
    /// Result containing the authenticated auth token
    /// High-level authentication method
    ///
    /// Convenience wrapper that authenticates using the client's configured secret.
    /// This is a Rust-specific helper method - JS SDK uses requestAuthToken directly.
    ///
    /// # Parameters
    /// - `meta`: Optional metadata for authorization (unused in current implementation)
    ///
    /// # Returns
    /// Authentication token
    pub async fn authenticate(&mut self, _meta: HashMap<String, serde_json::Value>) -> Result<AuthToken> {
        // Clone values to avoid borrow checker issues
        let secret = self.secret.clone();
        let cell_slug = self.cell_slug.clone();
        let encrypt = Some(self.encrypt);

        // Call the dual-path auth token method
        let auth_token = self.request_auth_token(
            secret.as_deref(),
            None,
            cell_slug.as_deref(),
            encrypt
        ).await?;

        // Store token for current URI (maintain backward compatibility)
        if let Some(current_uri) = self.get_current_uri() {
            self.auth_token_objects.insert(current_uri, auth_token.clone());
        }

        self.log("info", "Authentication successful");

        Ok(auth_token)
    }
    
    /// Refresh the current authentication token (equivalent to refreshToken in JS)
    ///
    /// # Returns
    ///
    /// Result containing the refreshed auth token
    pub async fn refresh_token(&mut self) -> Result<AuthToken> {
        if let Some(ref current_token) = self.auth_token {
            if !current_token.is_expired() {
                // Token is still valid, return it
                return Ok(current_token.clone());
            }
        }
        
        // Token is expired or missing, request new authentication
        let meta = HashMap::new(); // Empty meta for refresh
        self.authenticate(meta).await
    }
    
    /// Check if the client is currently authenticated (equivalent to isAuthenticated in JS)
    ///
    /// # Returns
    ///
    /// True if authenticated with a valid token
    pub fn is_authenticated(&self) -> bool {
        if let Some(ref token) = self.auth_token {
            !token.is_expired()
        } else {
            false
        }
    }
    
    /// Get the current authentication token (equivalent to getAuthToken in JS)
    ///
    /// # Returns
    ///
    /// Optional reference to the current auth token
    pub fn get_auth_token(&self) -> Option<&AuthToken> {
        self.auth_token.as_ref()
    }
    
    /// Set an authentication token (equivalent to setAuthToken in JS)
    ///
    /// # Arguments
    ///
    /// * `token` - AuthToken to set as current
    pub fn set_auth_token(&mut self, token: AuthToken) {
        self.auth_token = Some(token.clone());
        
        // Store for current URI
        if let Some(current_uri) = self.get_current_uri() {
            self.auth_token_objects.insert(current_uri, token);
        }
    }
    
    /// Clear the current authentication token (equivalent to clearAuthToken in JS)
    pub fn clear_auth_token(&mut self) {
        self.auth_token = None;
        self.auth_token_objects.clear();
        self.log("info", "Authentication token cleared");
    }
    
    /// Auto-authenticate if needed for requests (equivalent to ensureAuth in JS)
    ///
    /// # Arguments
    ///
    /// * `meta` - Optional metadata for authentication
    ///
    /// # Returns
    ///
    /// Result ensuring the client is authenticated
    pub async fn ensure_authentication(&mut self, meta: Option<HashMap<String, serde_json::Value>>) -> Result<()> {
        // Skip if authentication is in progress
        if self.auth_in_process {
            return Ok(());
        }
        
        // Check if we need to authenticate
        if !self.is_authenticated() {
            self.log("info", "Auto-authenticating for request");
            self.authenticate(meta.unwrap_or_default()).await?;
        }
        
        Ok(())
    }
    
    /// Save authentication token to persistent storage (equivalent to saveAuth in JS)
    ///
    /// # Arguments
    ///
    /// * `storage_key` - Key for storing the token snapshot
    ///
    /// # Returns
    ///
    /// Result with the serialized token snapshot
    pub fn save_auth_token(&self, storage_key: &str) -> Result<String> {
        if let Some(ref token) = self.auth_token {
            let snapshot = token.get_snapshot();
            let serialized = serde_json::to_string(&snapshot)?;
            
            // In a real implementation, you would save to persistent storage
            // For now, we just return the serialized data
            self.log("info", &format!("Auth token saved with key: {}", storage_key));
            
            Ok(serialized)
        } else {
            Err(KnishIOError::custom("No authentication token to save"))
        }
    }
    
    /// Load authentication token from persistent storage (equivalent to loadAuth in JS)
    ///
    /// # Arguments
    ///
    /// * `storage_key` - Key for retrieving the token snapshot
    /// * `serialized_data` - Serialized token data
    ///
    /// # Returns
    ///
    /// Result with the restored auth token
    pub fn load_auth_token(&mut self, storage_key: &str, serialized_data: &str) -> Result<AuthToken> {
        // Deserialize the token snapshot
        let snapshot: crate::auth::AuthTokenSnapshot = serde_json::from_str(serialized_data)?;
        
        // Get secret for restoration
        let secret = self.secret.as_deref()
            .ok_or_else(|| KnishIOError::custom("Secret must be set before loading auth token"))?;
            
        // Restore the auth token
        let restored_token = AuthToken::restore(snapshot, secret)?;
        
        // Set as current token
        self.set_auth_token(restored_token.clone());
        
        self.log("info", &format!("Auth token loaded with key: {}", storage_key));
        
        Ok(restored_token)
    }
    
    /// Get authentication token for a specific URI (equivalent to getAuthTokenForUri in JS)
    ///
    /// # Arguments
    ///
    /// * `uri` - URI to get token for
    ///
    /// # Returns
    ///
    /// Optional reference to the auth token for the URI
    pub fn get_auth_token_for_uri(&self, uri: &str) -> Option<&AuthToken> {
        self.auth_token_objects.get(uri)
    }
    
    /// Check if authentication is in progress (equivalent to isAuthInProgress in JS)
    ///
    /// # Returns
    ///
    /// True if authentication is currently in progress
    pub fn is_auth_in_progress(&self) -> bool {
        self.auth_in_process
    }
    
    /// Get the current URI being used
    ///
    /// # Returns
    ///
    /// Optional current URI string
    pub fn get_current_uri(&self) -> Option<String> {
        self.uris.get(self.current_uri_index).cloned()
    }
    
    /// Set the user secret for cryptographic operations
    ///
    /// # Arguments
    ///
    /// * `secret` - User secret key
    pub fn set_secret<S: Into<String>>(&mut self, secret: S) {
        let secret_string = secret.into();
        self.secret = Some(secret_string.clone());
        
        // Generate bundle hash from secret
        self.bundle = Some(crate::crypto::generate_bundle_hash(&secret_string));
        
        self.log("info", "User secret and bundle configured");
    }
    
    /// Set the encryption flag
    ///
    /// # Arguments
    ///
    /// * `encrypt` - Whether to enable ML-KEM quantum encryption
    pub fn set_encrypt(&mut self, encrypt: bool) {
        self.encrypt = encrypt;
        self.log("info", &format!("Encryption {}", if encrypt { "enabled" } else { "disabled" }));
    }
    
    // set_cell_slug already exists above
    
    /// Get the server SDK version
    ///
    /// # Returns
    ///
    /// The configured server SDK version
    pub fn get_server_sdk_version(&self) -> u32 {
        self.server_sdk_version
    }
    
    // =================== Query Methods ===================

    /// Execute a query or mutation with automatic auth token refresh
    ///
    /// Matches TS executeQuery(query, variables) at lines 474-487
    ///
    /// # Parameters
    /// - `query`: The query or mutation to execute
    /// - `variables`: Optional variables for the query
    ///
    /// # Returns
    /// Response from the query execution
    pub async fn execute_query<Q: crate::query::Query + ?Sized>(
        &mut self,
        query: &Q,
        variables: Option<serde_json::Value>
    ) -> Result<Box<dyn Response>> {
        // Check and refresh authorization token if needed (matches TS lines 476-483)
        if let Some(ref auth_token) = self.auth_token {
            if auth_token.is_expired() {
                self.log("info", "KnishIOClient::execute_query() - Access token is expired. Getting new one...");

                // Refresh the token (matches TS line 478)
                let secret = self.secret.clone();
                let cell_slug = self.cell_slug.clone();
                let encrypt = self.encrypt;

                let _new_token = self.request_auth_token(
                    secret.as_deref(),
                    None,
                    cell_slug.as_deref(),
                    Some(encrypt)
                ).await?;
            }
        }

        // Execute the query (matches TS line 486)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        query.execute(client, variables, None).await
    }

    /// Cancel a specific query
    ///
    /// Matches TS cancelQuery(query, variables) at lines 681-689
    ///
    /// # Parameters
    /// - `query_name`: Name of the query to cancel
    /// - `variables`: Variables used for the query (for key generation)
    pub fn cancel_query(&self, query_name: &str, variables: Option<serde_json::Value>) {
        // Generate query key (matches TS line 682)
        let query_key = format!("{}_{}", query_name,
            serde_json::to_string(&variables.unwrap_or(serde_json::json!({}))).unwrap_or_default());

        // Abort and remove controller (matches TS lines 683-688)
        if let Ok(mut controllers) = self.abort_controllers.lock() {
            if controllers.contains_key(&query_key) {
                controllers.remove(&query_key);
            }
        }
    }

    /// Cancel all pending queries
    ///
    /// Matches TS cancelAllQueries() at lines 694-699
    pub fn cancel_all_queries(&self) {
        // Abort all controllers and clear (matches TS lines 695-698)
        if let Ok(mut controllers) = self.abort_controllers.lock() {
            controllers.clear();
        }
    }

    /// Query balance for a wallet or token
    ///
    /// # Parameters
    /// - `token`: Token slug to query
    /// - `bundle_hash`: Optional bundle hash to query specific wallet
    ///
    /// # Returns
    /// Balance information for the specified wallet/token
    pub async fn query_balance(&self, token: &str, bundle_hash: Option<&str>) -> Result<Wallet> {
        use crate::query::balance::QueryBalance;
        use crate::query::Query;

        let mut query = QueryBalance::new()
            .with_token(token);

        if let Some(bundle) = bundle_hash {
            query = query.with_bundle_hash(bundle);
        } else if let Some(ref bundle) = self.bundle {
            query = query.with_bundle_hash(bundle);
        }

        // Execute query through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;

            // Extract wallet data from response
            // The response should contain wallet balance information
            let response_data = response.data();

            // Convert response data to Wallet
            // ResponseBalance.payload() returns Wallet in JS implementation
            if let Some(balance_data) = response_data.get("Balance") {
                // Use existing from_response_data method
                let wallet = Wallet::from_response_data(balance_data.clone())?;
                return Ok(wallet);
            }

            Err(KnishIOError::InvalidResponse)
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query wallets by bundle or token
    ///
    /// # Parameters
    /// - `bundle_hash`: Optional bundle hash to filter wallets
    /// - `token`: Optional token to filter wallets
    ///
    /// # Returns
    /// List of wallets matching the criteria
    pub async fn query_wallets(&self, bundle_hash: Option<&str>, token: Option<&str>) -> Result<Vec<Wallet>> {
        use crate::query::wallet_list::QueryWalletList;
        use crate::query::Query;

        let mut query = QueryWalletList::new();

        if let Some(bundle) = bundle_hash {
            query = query.with_bundle_hash(bundle);
        } else if let Some(ref bundle) = self.bundle {
            query = query.with_bundle_hash(bundle);
        }

        if let Some(t) = token {
            query = query.with_token_slug(t);
        }

        // Execute query through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Extract wallet list from response
            if let Some(wallets_data) = response_data.get("WalletList").and_then(|v| v.as_array()) {
                let wallets: Result<Vec<Wallet>> = wallets_data
                    .iter()
                    .map(|wallet_data| Wallet::from_response_data(wallet_data.clone()))
                    .collect();
                return wallets;
            }

            Ok(vec![])
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query bundle information
    ///
    /// # Parameters
    /// - `bundle_hash`: Bundle hash to query
    ///
    /// # Returns
    /// Bundle information including all associated wallets
    pub async fn query_bundle(&self, bundle_hash: Option<&str>) -> Result<serde_json::Value> {
        use crate::query::wallet_bundle::QueryWalletBundle;
        use crate::query::Query;

        // Get bundle hash - from parameter or client bundle
        let bundle = bundle_hash.or(self.bundle.as_deref())
            .ok_or(KnishIOError::MissingBundle)?;

        // Convert string to Vec (matching JS logic: bundle = [bundle])
        let bundle_hashes = vec![bundle.to_string()];

        let query = QueryWalletBundle::with_bundle_hashes(bundle_hashes);

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return WalletBundle data
            if let Some(bundle_data) = response_data.get("WalletBundle") {
                return Ok(bundle_data.clone());
            }

            Ok(serde_json::json!(null))
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query atoms based on comprehensive criteria (matches JS queryAtom)
    ///
    /// # Parameters
    /// All parameters are optional - pass only those you want to filter by
    ///
    /// # Returns
    /// List of atoms matching the criteria
    pub async fn query_atom(
        &self,
        molecular_hash: Option<&str>,
        bundle_hash: Option<&str>,
        position: Option<&str>,
        wallet_address: Option<&str>,
        isotope: Option<&str>,
        token_slug: Option<&str>,
        batch_id: Option<&str>,
        meta_type: Option<&str>,
        meta_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        use crate::query::atom::QueryAtom;
        use crate::query::Query;

        let mut query = QueryAtom::new();

        // Add filters based on provided parameters (matching JS logic)
        if let Some(hash) = molecular_hash {
            query = query.add_molecular_hash(hash);
        }
        if let Some(bundle) = bundle_hash {
            query = query.add_bundle_hash(bundle);
        }
        if let Some(pos) = position {
            query = query.add_position(pos);
        }
        if let Some(addr) = wallet_address {
            query = query.add_wallet_address(addr);
        }
        if let Some(iso) = isotope {
            query = query.add_isotope(iso);
        }
        if let Some(token) = token_slug {
            query = query.add_token_slug(token);
        }
        if let Some(batch) = batch_id {
            query = query.add_batch_id(batch);
        }
        if let Some(m_type) = meta_type {
            query = query.add_meta_type(m_type);
        }
        if let Some(m_id) = meta_id {
            query = query.add_meta_id(m_id);
        }

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return atom array
            if let Some(atoms_data) = response_data.get("Atom").and_then(|v| v.as_array()) {
                return Ok(atoms_data.clone());
            }

            Ok(vec![])
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query batch information
    ///
    /// # Parameters
    /// - `batch_id`: Batch ID to query
    ///
    /// # Returns
    /// Batch information including all transactions
    pub async fn query_batch(&self, batch_id: &str) -> Result<serde_json::Value> {
        use crate::query::batch::QueryBatch;
        use crate::query::Query;

        let query = QueryBatch::with_batch_id(batch_id);

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return batch data
            if let Some(batch_data) = response_data.get("Batch") {
                return Ok(batch_data.clone());
            }

            Ok(serde_json::json!(null))
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query batch history (matches JS queryBatchHistory)
    ///
    /// # Parameters
    /// - `batch_id`: Batch ID to filter history
    ///
    /// # Returns
    /// List of batch history entries
    pub async fn query_batch_history(&self, batch_id: &str) -> Result<Vec<serde_json::Value>> {
        use crate::query::batch_history::QueryBatchHistory;
        use crate::query::Query;

        let query = QueryBatchHistory::with_batch_id(batch_id);

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return batch history array
            if let Some(history_data) = response_data.get("BatchHistory").and_then(|v| v.as_array()) {
                return Ok(history_data.clone());
            }

            Ok(vec![])
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query source wallet with sufficient balance for token operations
    ///
    /// This is a critical method used by transfer, burn, and other token operations
    /// to ensure the source wallet has sufficient balance.
    ///
    /// # Parameters
    /// - `token`: Token slug to query
    /// - `amount`: Required amount
    /// - `wallet_type`: Optional wallet type (defaults to "regular")
    ///
    /// # Returns
    /// Wallet with sufficient balance
    ///
    /// # Errors
    /// Returns `TransferBalance` error if insufficient balance or shadow wallet
    pub async fn query_source_wallet(&self, token: &str, amount: f64, wallet_type: Option<&str>) -> Result<Wallet> {
        let _wallet_type = wallet_type.unwrap_or("regular");

        // Query balance for this token
        let source_wallet = self.query_balance(token, None).await?;

        // Check if we have enough tokens (i128 for precision-safe comparison)
        if source_wallet.balance_as_i128() < (amount as i128) {
            return Err(KnishIOError::TransferBalance);
        }

        // Check for shadow wallet (no position or address)
        if source_wallet.position.is_none() || source_wallet.address.is_none() {
            return Err(KnishIOError::WalletCredential);
        }

        Ok(source_wallet)
    }

    /// Query ContinuID information
    ///
    /// # Parameters
    /// - `bundle_hash`: Bundle hash to query ContinuID for
    ///
    /// # Returns
    /// ContinuID information including position chain
    pub async fn query_continu_id(&self, bundle_hash: Option<&str>) -> Result<Option<Wallet>> {
        use crate::query::continu_id::QueryContinuId;
        use crate::query::Query;

        let bundle = bundle_hash.or(self.bundle.as_deref())
            .ok_or(KnishIOError::MissingBundle)?;

        let query = QueryContinuId::new(bundle);

        // Execute query through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Extract ContinuID wallet from response
            if let Some(continuid_data) = response_data.get("ContinuId") {
                if continuid_data.is_null() {
                    return Ok(None);
                }
                let wallet = Wallet::from_response_data(continuid_data.clone())?;
                return Ok(Some(wallet));
            }

            Ok(None)
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query policy information
    ///
    /// # Parameters
    /// - `meta_type`: Meta type for the policy
    /// - `meta_id`: Meta ID for the policy
    ///
    /// # Returns
    /// Policy details and rules
    pub async fn query_policy(&self, meta_type: &str, meta_id: &str) -> Result<serde_json::Value> {
        use crate::query::policy::QueryPolicy;
        use crate::query::Query;

        let query = QueryPolicy::new()
            .with_meta_type(meta_type)
            .with_meta_id(meta_id);

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return policy data
            if let Some(policy_data) = response_data.get("Policy") {
                return Ok(policy_data.clone());
            }

            Ok(serde_json::json!(null))
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query user activity with comprehensive filtering (matches JS queryUserActivity)
    ///
    /// # Parameters
    /// All parameters are optional - pass only those you want to filter by
    ///
    /// # Returns
    /// List of user activity entries
    pub async fn query_user_activity(
        &self,
        bundle_hash: Option<&str>,
        meta_type: Option<&str>,
        meta_id: Option<&str>,
        ip_address: Option<&str>,
        browser: Option<&str>,
        os_cpu: Option<&str>,
        resolution: Option<&str>,
        time_zone: Option<&str>,
        count_by: Option<Vec<String>>,
        interval: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        use crate::query::user_activity::QueryUserActivity;
        use crate::query::Query;

        let mut query = QueryUserActivity::new();

        // Configure all optional parameters (matching JS logic)
        if let Some(bundle) = bundle_hash {
            query = query.with_bundle_hash(bundle);
        }
        if let Some(m_type) = meta_type {
            query = query.with_meta_type(m_type);
        }
        if let Some(m_id) = meta_id {
            query = query.with_meta_id(m_id);
        }
        if let Some(ip) = ip_address {
            query = query.with_ip_address(ip);
        }
        if let Some(br) = browser {
            query = query.with_browser(br);
        }
        if let Some(os) = os_cpu {
            query = query.with_os_cpu(os);
        }
        if let Some(res) = resolution {
            query = query.with_resolution(res);
        }
        if let Some(tz) = time_zone {
            query = query.with_time_zone(tz);
        }
        if let Some(cb) = count_by {
            query = query.with_count_by(cb);
        }
        if let Some(int) = interval {
            query = query.with_interval(int);
        }

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return activity array
            if let Some(activity_data) = response_data.get("UserActivity").and_then(|v| v.as_array()) {
                return Ok(activity_data.clone());
            }

            Ok(vec![])
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query active session information (matches JS queryActiveSession)
    ///
    /// # Parameters
    /// - `bundle_hash`: Bundle hash for session lookup
    /// - `meta_type`: Meta type for the session
    /// - `meta_id`: Meta ID for the session
    ///
    /// # Returns
    /// Active session information
    pub async fn query_active_session(
        &self,
        bundle_hash: Option<&str>,
        meta_type: Option<&str>,
        meta_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        use crate::query::active_session::QueryActiveSession;
        use crate::query::Query;

        let mut query = QueryActiveSession::new();

        // Configure optional parameters (matching JS logic)
        if let Some(bundle) = bundle_hash {
            query = query.with_bundle_hash(bundle);
        }
        if let Some(m_type) = meta_type {
            query = query.with_meta_type(m_type);
        }
        if let Some(m_id) = meta_id {
            query = query.with_meta_id(m_id);
        }

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return active session data
            if let Some(session_data) = response_data.get("ActiveSession") {
                return Ok(session_data.clone());
            }

            Ok(serde_json::json!(null))
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query token information (used for fungibility checks - matches JS internal usage)
    ///
    /// # Parameters
    /// - `slug`: Token slug to query
    ///
    /// # Returns
    /// Token information including fungibility
    pub async fn query_token(&self, slug: &str) -> Result<serde_json::Value> {
        use crate::query::token::QueryToken;
        use crate::query::Query;

        let query = QueryToken::new()
            .with_slug(slug);

        // Execute through GraphQL client
        if let Some(ref client) = self.client {
            let response = query.execute(client, None, None).await?;
            let response_data = response.data();

            // Return token data (usually an array, get first element like JS does)
            if let Some(token_data) = response_data.get("Token") {
                return Ok(token_data.clone());
            }

            Ok(serde_json::json!(null))
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Query metadata with dual-path logic (matches JS queryMeta)
    ///
    /// # Parameters
    /// - `meta_type`: Meta type to query
    /// - `meta_id`: Optional meta ID
    /// - `key`: Optional meta key filter
    /// - `value`: Optional meta value filter
    /// - `through_atom`: If true, use QueryMetaTypeViaAtom; if false, use QueryMetaType (default: true)
    ///
    /// # Returns
    /// Metadata matching the criteria
    pub async fn query_meta(
        &self,
        meta_type: &str,
        meta_id: Option<&str>,
        key: Option<&str>,
        value: Option<&str>,
        through_atom: Option<bool>,
    ) -> Result<serde_json::Value> {
        use crate::query::Query;

        let use_atom = through_atom.unwrap_or(true);

        // Dual-path logic: QueryMetaTypeViaAtom or QueryMetaType
        if use_atom {
            use crate::query::meta_type_via_atom::QueryMetaTypeViaAtom;

            let mut query = QueryMetaTypeViaAtom::new()
                .add_meta_type(meta_type);

            if let Some(id) = meta_id {
                query = query.add_meta_id(id);
            }
            if let Some(k) = key {
                query = query.add_key(k);
            }
            if let Some(v) = value {
                query = query.add_value(v);
            }
            if let Some(ref cell) = self.cell_slug {
                query = query.add_cell_slug(cell);
            }

            // Execute through GraphQL client
            if let Some(ref client) = self.client {
                let response = query.execute(client, None, None).await?;
                let response_data = response.data();

                if let Some(meta_data) = response_data.get("MetaType") {
                    return Ok(meta_data.clone());
                }

                Ok(serde_json::json!(null))
            } else {
                Err(KnishIOError::NoClient)
            }
        } else {
            use crate::query::meta_type::QueryMetaType;

            let mut query = QueryMetaType::new()
                .with_meta_type(meta_type);

            if let Some(id) = meta_id {
                query = query.with_meta_id(id);
            }
            if let Some(k) = key {
                query = query.with_key(k);
            }
            if let Some(v) = value {
                query = query.with_value(v);
            }
            if let Some(ref cell) = self.cell_slug {
                query = query.with_cell_slug(cell);
            }

            // Execute through GraphQL client
            if let Some(ref client) = self.client {
                let response = query.execute(client, None, None).await?;
                let response_data = response.data();

                if let Some(meta_data) = response_data.get("MetaType") {
                    return Ok(meta_data.clone());
                }

                Ok(serde_json::json!(null))
            } else {
                Err(KnishIOError::NoClient)
            }
        }
    }

    // =================== Creation Methods ===================

    /// Create a new wallet
    ///
    /// Matches JS createWallet({ token }) at lines 1010-1028
    ///
    /// # Parameters
    /// - `token`: Token slug for the wallet
    ///
    /// # Returns
    /// Response from wallet creation mutation
    pub async fn create_wallet(&mut self, token: &str) -> Result<Box<dyn Response>> {
        use crate::mutation::create_wallet::MutationCreateWallet;
        use crate::mutation::Mutation;

        // Create new wallet (matches JS line 1013-1016)
        let new_wallet = Wallet::new(
            Some(&self.secret.as_ref().ok_or(KnishIOError::MissingSecret)?),
            None,
            Some(token),
            None,
            None,
            None,
            None,
        )?;

        // Create mutation (matches JS lines 1021-1023)
        let mut mutation = MutationCreateWallet::from_molecule(Molecule::new());

        // Fill molecule with wallet (matches JS line 1025)
        mutation.fill_molecule(&new_wallet)?;

        // Execute mutation (matches JS line 1027)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Create a new token
    ///
    /// # Parameters
    /// - `token`: Token slug
    /// - `amount`: Initial amount to create
    /// - `meta`: Token metadata
    ///
    /// # Returns
    /// Token creation response
    /// Create a new token with the given parameters
    ///
    /// Matches JS createToken({ token, amount, meta, batchId, units }) at lines 1152-1208
    ///
    /// # Parameters
    /// - `token`: Token identifier
    /// - `amount`: Amount of tokens to create (optional)
    /// - `meta`: Metadata for the token (optional)
    /// - `batch_id`: Batch ID for stackable tokens (optional)
    /// - `units`: Unit IDs for nonfungible/stackable tokens
    ///
    /// # Returns
    /// Token creation response
    pub async fn create_token(
        &mut self,
        token: &str,
        mut amount: Option<f64>,
        mut meta: Option<HashMap<String, Value>>,
        batch_id: Option<&str>,
        units: Vec<String>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::create_token::{MutationCreateToken, CreateTokenParams};
        use crate::mutation::Mutation;
        use crate::crypto::generate_batch_id;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        // Get fungibility mode from meta (matches JS line 1160)
        let fungibility = meta.as_ref()
            .and_then(|m| m.get("fungibility"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // For stackable tokens - create a batch ID (matches JS lines 1163-1165)
        let final_batch_id = if fungibility == "stackable" {
            if let Some(bid) = batch_id {
                Some(bid.to_string())
            } else {
                // Generate batch ID using crypto utility
                Some(generate_batch_id())
            }
        } else {
            batch_id.map(|s| s.to_string())
        };

        // Special logic for token unit initialization (nonfungible || stackable) (matches JS lines 1168-1184)
        if (fungibility == "nonfungible" || fungibility == "stackable") && !units.is_empty() {
            // Stackable tokens with Unit IDs must not use decimals (matches JS lines 1170-1172)
            let decimals = meta.as_ref()
                .and_then(|m| m.get("decimals"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            if decimals > 0.0 {
                return Err(KnishIOError::StackableUnitDecimals);
            }

            // Can't create stackable units AND provide amount (matches JS lines 1175-1177)
            if amount.unwrap_or(0.0) > 0.0 {
                return Err(KnishIOError::StackableUnitAmount);
            }

            // Calculating amount based on Unit IDs (matches JS lines 1180-1183)
            amount = Some(units.len() as f64);

            // Update meta
            let mut meta_map = meta.unwrap_or_default();
            meta_map.insert("splittable".to_string(), Value::from(1));
            meta_map.insert("decimals".to_string(), Value::from(0));
            meta_map.insert("tokenUnits".to_string(), Value::String(serde_json::to_string(&units)?));
            meta = Some(meta_map);
        }

        // Add batchId to meta if it was set (matches JS line 1164)
        if let Some(ref bid) = final_batch_id {
            let mut meta_map = meta.unwrap_or_default();
            meta_map.insert("batchId".to_string(), Value::String(bid.clone()));
            meta = Some(meta_map);
        }

        // Creating the wallet that will receive the new tokens (matches JS lines 1187-1192)
        let recipient_wallet = Wallet::new(
            Some(self.secret.as_ref().ok_or(KnishIOError::MissingSecret)?),
            Some(&self.bundle.as_ref().ok_or(KnishIOError::MissingBundle)?),
            Some(token),
            final_batch_id.as_deref(),
            None,
            None,
            None,
        )?;

        // Create mutation (matches JS lines 1197-1199)
        let mut mutation = MutationCreateToken::from_molecule(Molecule::new());

        // Fill molecule (matches JS lines 1201-1205)
        mutation.fill_molecule(CreateTokenParams {
            recipient_wallet,
            amount: amount.unwrap_or(0.0),
            meta,
        })?;

        // Execute mutation (matches JS line 1207)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Transfer tokens between wallets
    ///
    /// # Parameters
    /// - `recipient`: Recipient wallet address or bundle hash
    /// - `token`: Token slug to transfer
    /// - `amount`: Amount to transfer
    ///
    /// # Returns
    /// Transfer response
    /// Transfer tokens between wallets
    ///
    /// Matches JS transferToken({ bundleHash, token, amount, units, batchId, sourceWallet }) at lines 1640-1717
    ///
    /// # Parameters
    /// - `bundle_hash`: Recipient bundle hash
    /// - `token`: Token slug to transfer
    /// - `amount`: Amount to transfer (optional if units provided)
    /// - `units`: Token units to transfer (optional)
    /// - `batch_id`: Batch ID for recipient (optional)
    /// - `source_wallet`: Source wallet (optional, will be queried if not provided)
    ///
    /// # Returns
    /// Transfer response
    pub async fn transfer_token(
        &mut self,
        bundle_hash: &str,
        token: &str,
        mut amount: Option<f64>,
        units: Vec<String>,
        batch_id: Option<&str>,
        source_wallet: Option<Wallet>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::transfer_tokens::{MutationTransferTokens, TransferTokensParams};
        use crate::mutation::Mutation;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        // Calculate amount & set meta key (matches JS lines 1649-1656)
        if !units.is_empty() {
            // Can't move stackable units AND provide amount
            if amount.unwrap_or(0.0) > 0.0 {
                return Err(KnishIOError::StackableUnitAmount);
            }

            amount = Some(units.len() as f64);
        }

        // Get a source wallet (matches JS lines 1659-1664)
        let mut source_wallet = if let Some(wallet) = source_wallet {
            wallet
        } else {
            self.query_source_wallet(token, amount.unwrap_or(0.0), None).await?
        };

        // Do you have enough tokens? (i128 for precision-safe comparison)
        if source_wallet.balance_as_i128() < (amount.unwrap_or(0.0) as i128) {
            return Err(KnishIOError::TransferBalance);
        }

        // Attempt to get the recipient's wallet (matches JS lines 1672-1675)
        let mut recipient_wallet = Wallet::create(
            None,
            Some(bundle_hash),
            token,
            None,
            None,
        )?;

        // Compute the batch ID for the recipient (matches JS lines 1678-1685)
        if let Some(bid) = batch_id {
            recipient_wallet.batch_id = Some(bid.to_string());
        } else {
            recipient_wallet.init_batch_id(Some(&source_wallet), false);
        }

        // Create a remainder from the source wallet (matches JS line 1688)
        let secret = self.secret.as_ref()
            .ok_or(KnishIOError::MissingSecret)?;
        let mut remainder_wallet = source_wallet.create_remainder(secret)?;

        // Token units splitting (matches JS lines 1691-1695)
        if !units.is_empty() {
            source_wallet.split_units(&units, &mut remainder_wallet, Some(&mut recipient_wallet));
        }

        // Build the molecule itself (matches JS lines 1699-1702)
        let mut molecule = Molecule::new();
        molecule.source_wallet = Some(source_wallet.clone());
        molecule.remainder_wallet = Some(remainder_wallet.clone());

        // Create mutation (matches JS lines 1706-1709)
        let mut mutation = MutationTransferTokens::from_molecule(molecule);

        // Fill molecule (matches JS lines 1711-1714)
        mutation.fill_molecule(TransferTokensParams {
            recipient_wallet,
            amount: amount.unwrap_or(0.0),
        })?;

        // Execute mutation (matches JS line 1716)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Request tokens (minting)
    ///
    /// Matches JS requestTokens({ token, to, amount, units, meta, batchId }) at lines 1471-1558
    ///
    /// # Parameters
    /// - `token`: Token slug to request
    /// - `to`: Recipient (BundleHash, Secret, Wallet, or None for self)
    /// - `amount`: Amount to request (optional if units provided)
    /// - `units`: Token units (optional)
    /// - `meta`: Metadata (optional)
    /// - `batch_id`: Batch ID for stackable tokens (optional)
    ///
    /// # Returns
    /// Token request response
    pub async fn request_tokens(
        &mut self,
        token: &str,
        to: Option<RecipientType>,
        mut amount: Option<f64>,
        units: Vec<String>,
        meta: Option<HashMap<String, Value>>,
        batch_id: Option<&str>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::request_tokens::{MutationRequestTokens, RequestTokensParams};
        use crate::mutation::Mutation;
        use crate::crypto::generate_batch_id;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        // Initialize meta (matches JS line 1482)
        let mut meta_map = meta.unwrap_or_default();

        // Get token & check fungibility (matches JS lines 1485-1489)
        let token_response = self.query_token(token).await?;
        let is_stackable = token_response
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|obj| obj.get("fungibility"))
            .and_then(|f| f.as_str())
            .map(|f| f == "stackable")
            .unwrap_or(false);

        // Batch ID validation (matches JS lines 1491-1498)
        let final_batch_id = if !is_stackable && batch_id.is_some() {
            // NON-stackable tokens & batch ID is NOT NULL - error
            return Err(KnishIOError::BatchId);
        } else if is_stackable && batch_id.is_none() {
            // Stackable tokens & batch ID is NULL - generate new one
            Some(generate_batch_id())
        } else {
            batch_id.map(|s| s.to_string())
        };

        // Calculate amount & set meta key (matches JS lines 1501-1510)
        if !units.is_empty() {
            // Can't move stackable units AND provide amount
            if amount.unwrap_or(0.0) > 0.0 {
                return Err(KnishIOError::StackableUnitAmount);
            }

            // Calculating amount based on Unit IDs
            amount = Some(units.len() as f64);
            meta_map.insert("tokenUnits".to_string(), Value::String(serde_json::to_string(&units)?));
        }

        // Recipient routing logic (matches JS lines 1512-1539)
        let (meta_type, meta_id) = if let Some(recipient) = to {
            match recipient {
                // String + isBundleHash → walletBundle
                RecipientType::BundleHash(bundle) => {
                    ("walletBundle".to_string(), bundle)
                }

                // String + NOT bundle → create wallet from secret
                RecipientType::Secret(secret) => {
                    let wallet = Wallet::create(
                        Some(&secret),
                        None,
                        token,
                        None,
                        None,
                    )?;

                    meta_map.insert("position".to_string(), Value::String(wallet.position.clone().unwrap_or_default()));
                    meta_map.insert("bundle".to_string(), Value::String(wallet.bundle.clone().unwrap_or_default()));

                    ("wallet".to_string(), wallet.address.unwrap_or_default())
                }

                // Wallet instance → wallet with position/bundle
                RecipientType::Wallet(wallet) => {
                    meta_map.insert("position".to_string(), Value::String(wallet.position.clone().unwrap_or_default()));
                    meta_map.insert("bundle".to_string(), Value::String(wallet.bundle.clone().unwrap_or_default()));

                    ("wallet".to_string(), wallet.address.unwrap_or_default())
                }
            }
        } else {
            // No recipient, so request tokens for ourselves
            ("walletBundle".to_string(), self.bundle.clone().ok_or(KnishIOError::MissingBundle)?)
        };

        // Create mutation (matches JS lines 1544-1546)
        let mut mutation = MutationRequestTokens::from_molecule(Molecule::new());

        // Fill molecule (matches JS lines 1548-1555)
        mutation.fill_molecule(RequestTokensParams {
            token: token.to_string(),
            amount: amount.unwrap_or(0.0),
            meta_type,
            meta_id,
            meta: Some(meta_map),
            batch_id: final_batch_id,
        })?;

        // Execute mutation (matches JS line 1557)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Burn tokens
    ///
    /// # Parameters
    /// - `token`: Token slug to burn
    /// - `amount`: Amount to burn
    ///
    /// # Returns
    /// Burn response
    /// Burn tokens
    ///
    /// Matches JS burnTokens({ token, amount, units, sourceWallet }) at lines 1824-1876
    ///
    /// # Parameters
    /// - `token`: Token slug to burn
    /// - `amount`: Amount to burn (optional if units provided)
    /// - `units`: Token units to burn (optional)
    /// - `source_wallet`: Source wallet (optional, will be queried if not provided)
    ///
    /// # Returns
    /// Burn response
    pub async fn burn_tokens(
        &mut self,
        token: &str,
        mut amount: Option<f64>,
        units: Vec<String>,
        source_wallet: Option<Wallet>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::propose_molecule::MutationProposeMolecule;
        use crate::mutation::Mutation;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        // Get a source wallet (matches JS lines 1831-1836)
        let mut source_wallet = if let Some(wallet) = source_wallet {
            wallet
        } else {
            self.query_source_wallet(token, amount.unwrap_or(0.0), None).await?
        };

        // Remainder wallet (matches JS line 1839)
        let secret = self.secret.as_ref()
            .ok_or(KnishIOError::MissingSecret)?;
        let mut remainder_wallet = source_wallet.create_remainder(secret)?;

        // Calculate amount & set meta key (matches JS lines 1842-1857)
        if !units.is_empty() {
            // Can't burn stackable units AND provide amount (matches JS lines 1844-1846)
            if amount.unwrap_or(0.0) > 0.0 {
                return Err(KnishIOError::StackableUnitAmount);
            }

            // Calculating amount based on Unit IDs (matches JS line 1849)
            amount = Some(units.len() as f64);

            // Token units splitting (matches JS lines 1852-1855)
            source_wallet.split_units(&units, &mut remainder_wallet, None);
        }

        // Create a molecule (matches JS lines 1860-1863)
        let mut molecule = Molecule::new();
        molecule.source_wallet = Some(source_wallet.clone());
        molecule.remainder_wallet = Some(remainder_wallet.clone());

        // Burn token (matches JS line 1864)
        molecule.burn_token(amount.unwrap_or(0.0), None)?;

        // Sign molecule (matches JS lines 1865-1867)
        let bundle = self.bundle.clone();
        molecule.sign(bundle, false, false)?;

        // Check molecule (matches JS line 1868)
        molecule.check(None)?;

        // Create & execute a mutation (matches JS lines 1871-1875)
        let mutation = MutationProposeMolecule::from_molecule(molecule);

        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Replenish token supply
    ///
    /// # Parameters
    /// - `token`: Token slug to replenish
    /// - `amount`: Amount to replenish
    ///
    /// # Returns
    /// Replenish response
    /// Replenish token supply
    ///
    /// Matches JS replenishToken({ token, amount, units, sourceWallet }) at lines 1887-1923
    ///
    /// # Parameters
    /// - `token`: Token slug to replenish
    /// - `amount`: Amount to replenish (optional if units provided)
    /// - `units`: Token units (optional)
    /// - `source_wallet`: Source wallet (optional, will be queried if not provided)
    ///
    /// # Returns
    /// Replenish response
    pub async fn replenish_token(
        &mut self,
        token: &str,
        amount: Option<f64>,
        units: Vec<String>,
        source_wallet: Option<Wallet>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::propose_molecule::MutationProposeMolecule;
        use crate::mutation::Mutation;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        // If no source wallet, query balance (matches JS lines 1893-1898)
        let source_wallet = if let Some(wallet) = source_wallet {
            wallet
        } else {
            // Query balance returns a Wallet directly
            self.query_balance(token, None).await?
        };

        // Check if wallet is valid (matches JS lines 1896-1898)
        if source_wallet.address.is_none() {
            return Err(KnishIOError::WalletCredential);
        }

        // Remainder wallet (matches JS line 1901)
        let secret = self.secret.as_ref()
            .ok_or(KnishIOError::MissingSecret)?;
        let remainder_wallet = source_wallet.create_remainder(secret)?;

        // Create a molecule (matches JS lines 1904-1907)
        let mut molecule = Molecule::new();
        molecule.source_wallet = Some(source_wallet.clone());
        molecule.remainder_wallet = Some(remainder_wallet.clone());

        // Replenish token (matches JS lines 1908-1911)
        molecule.replenish_token(amount.unwrap_or(0.0), Some(units))?;

        // Sign molecule (matches JS lines 1912-1914)
        let bundle = self.bundle.clone();
        molecule.sign(bundle, false, false)?;

        // Check molecule (matches JS line 1915)
        molecule.check(None)?;

        // Create & execute a mutation (matches JS lines 1918-1922)
        let mutation = MutationProposeMolecule::from_molecule(molecule);

        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Fuse fungible token units
    ///
    /// # Parameters
    /// - `token`: Token slug
    /// - `token_units`: List of token unit IDs to fuse
    ///
    /// # Returns
    /// Fuse response
    /// Fuse fungible token units
    ///
    /// Matches JS fuseToken({ bundleHash, tokenSlug, newTokenUnit, fusedTokenUnitIds, sourceWallet }) at lines 1934-2003
    ///
    /// # Parameters
    /// - `bundle_hash`: Recipient bundle hash
    /// - `token_slug`: Token slug
    /// - `new_token_unit`: New fused token unit
    /// - `fused_token_unit_ids`: List of token unit IDs to fuse
    /// - `source_wallet`: Source wallet (optional, will be queried if not provided)
    ///
    /// # Returns
    /// Fuse response
    pub async fn fuse_token(
        &mut self,
        bundle_hash: &str,
        token_slug: &str,
        mut new_token_unit: crate::token_unit::TokenUnit,
        fused_token_unit_ids: Vec<String>,
        source_wallet: Option<Wallet>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::propose_molecule::MutationProposeMolecule;
        use crate::mutation::Mutation;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        // Get source wallet (matches JS lines 1941-1943)
        let mut source_wallet = if let Some(wallet) = source_wallet {
            wallet
        } else {
            self.query_balance(token_slug, None).await?
        };

        // 3-Layer Validation (matches JS lines 1946-1954)

        // Layer 1: Source wallet exists (matches JS lines 1946-1948)
        if source_wallet.address.is_none() {
            return Err(KnishIOError::TransferBalance);
        }

        // Layer 2: Source wallet has token units (matches JS lines 1949-1951)
        if source_wallet.token_units.is_empty() {
            return Err(KnishIOError::TransferBalance);
        }

        // Layer 3: Fused token unit list not empty (matches JS lines 1952-1954)
        if fused_token_unit_ids.is_empty() {
            return Err(KnishIOError::TransferBalance);
        }

        // Validate all fused IDs exist in source (matches JS lines 1957-1965)
        let source_token_unit_ids: Vec<String> = source_wallet.token_units
            .iter()
            .map(|unit| unit.id.clone())
            .collect();

        for fused_id in &fused_token_unit_ids {
            if !source_token_unit_ids.contains(fused_id) {
                return Err(KnishIOError::TransferBalance);
            }
        }

        // Create recipient wallet (matches JS lines 1968-1971)
        let mut recipient_wallet = Wallet::create(
            None,
            Some(bundle_hash),
            token_slug,
            None,
            None,
        )?;

        // Set batch ID (matches JS line 1974)
        recipient_wallet.init_batch_id(Some(&source_wallet), false);

        // Create remainder wallet (matches JS line 1977)
        let secret = self.secret.as_ref()
            .ok_or(KnishIOError::MissingSecret)?;
        let mut remainder_wallet = source_wallet.create_remainder(secret)?;

        // Split token units (fused) - CRITICAL: Only to remainder, not recipient! (matches JS line 1980)
        source_wallet.split_units(&fused_token_unit_ids, &mut remainder_wallet, None);

        // Set recipient new fused token unit (matches JS lines 1983-1984)
        // CRITICAL: After split_units, source_wallet.token_units contains ONLY the fused units
        new_token_unit.metas.insert(
            "fusedTokenUnits".to_string(),
            serde_json::to_value(source_wallet.get_token_units_data())?
        );
        recipient_wallet.token_units = vec![new_token_unit];

        // Create a molecule (matches JS lines 1987-1990)
        let mut molecule = Molecule::new();
        molecule.source_wallet = Some(source_wallet.clone());
        molecule.remainder_wallet = Some(remainder_wallet);

        // Fuse token (matches JS line 1991)
        // Extract IDs from token units (after split_units, source_wallet contains only fused units)
        let fused_ids: Vec<String> = source_wallet.token_units.iter()
            .map(|unit| unit.id.clone())
            .collect();
        molecule.fuse_token(fused_ids, &recipient_wallet)?;

        // Sign molecule (matches JS lines 1992-1994)
        let bundle = self.bundle.clone();
        molecule.sign(bundle, false, false)?;

        // Check molecule (matches JS line 1995)
        molecule.check(None)?;

        // Create & execute a mutation (matches JS lines 1998-2002)
        let mutation = MutationProposeMolecule::from_molecule(molecule);

        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Deposit tokens to buffer
    ///
    /// Matches TS depositBufferToken({ tokenSlug, amount, tradeRates, sourceWallet }) at lines 1830-1872
    ///
    /// # Parameters
    /// - `token`: Token slug
    /// - `amount`: Amount to deposit
    /// - `trade_rates`: Trade rates for the buffer deposit
    /// - `source_wallet`: Optional source wallet (will be queried if not provided)
    ///
    /// # Returns
    /// Deposit response
    pub async fn deposit_buffer_token(
        &mut self,
        token: &str,
        amount: f64,
        trade_rates: std::collections::HashMap<String, f64>,
        source_wallet: Option<Wallet>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::deposit_buffer_token::{MutationDepositBufferToken, DepositBufferTokenParams};
        use crate::mutation::Mutation;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        self.log("info", &format!("KnishIOClient::deposit_buffer_token() - Depositing {} of {} to buffer...", amount, token));

        // Get source wallet if not provided (matches TS lines 1844-1849)
        let source_wallet = if let Some(wallet) = source_wallet {
            wallet
        } else {
            self.query_source_wallet(token, amount, None).await?
        };

        // Create molecule with source wallet
        let mut molecule = Molecule::new();
        molecule.source_wallet = Some(source_wallet);

        // Create mutation (matches TS line 1851)
        let mut mutation = MutationDepositBufferToken::from_molecule(molecule);

        // Fill molecule (matches TS lines 1854-1859)
        mutation.fill_molecule(DepositBufferTokenParams {
            amount,
            trade_rates,
        })?;

        // Execute mutation (matches TS line 1865)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Withdraw tokens from buffer
    ///
    /// Matches TS withdrawBufferToken({ tokenSlug, amount, sourceWallet, signingWallet }) at lines 1877-1916
    ///
    /// # Parameters
    /// - `token`: Token slug
    /// - `amount`: Amount to withdraw
    /// - `source_wallet`: Optional source wallet (will use default if not provided)
    /// - `signing_wallet`: Optional signing wallet for the withdrawal
    ///
    /// # Returns
    /// Withdrawal response
    pub async fn withdraw_buffer_token(
        &mut self,
        token: &str,
        amount: f64,
        source_wallet: Option<Wallet>,
        signing_wallet: Option<Wallet>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::withdraw_buffer_token::{MutationWithdrawBufferToken, WithdrawBufferTokenParams};
        use crate::mutation::Mutation;

        // Ensure we have authentication
        self.ensure_authentication(None).await?;

        self.log("info", &format!("KnishIOClient::withdraw_buffer_token() - Withdrawing {} of {} from buffer...", amount, token));

        // Get source wallet if not provided (matches TS lines 1891-1893)
        let source_wallet = if let Some(wallet) = source_wallet {
            wallet
        } else {
            self.get_source_wallet().await?
        };

        // Create molecule with source wallet
        let mut molecule = Molecule::new();
        molecule.source_wallet = Some(source_wallet);

        // Create mutation (matches TS line 1895)
        let mut mutation = MutationWithdrawBufferToken::from_molecule(molecule);

        // Fill molecule (matches TS lines 1898-1903 and JS lines 1806-1811)
        // Create recipients map: bundle -> amount (matches JS lines 1806-1807)
        let mut recipients = std::collections::HashMap::new();
        let bundle = self.get_bundle()
            .ok_or(KnishIOError::MissingBundle)?;
        recipients.insert(bundle.to_string(), amount);

        mutation.fill_molecule(WithdrawBufferTokenParams {
            recipients,
            signing_wallet,
        })?;

        // Execute mutation (matches TS line 1909)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Claim shadow wallet (equivalent to claimShadowWallet in JS)
    ///
    /// Matches JS claimShadowWallet({ token, batchId, molecule }) at lines 1569-1588
    ///
    /// # Parameters
    /// - `token`: Token slug of shadow wallet
    /// - `batch_id`: Optional batch ID for the claim
    /// - `molecule`: Optional molecule to use (if not provided, will create one)
    ///
    /// # Returns
    /// Response from claiming the shadow wallet
    pub async fn claim_shadow_wallet(
        &mut self,
        token: &str,
        batch_id: Option<&str>,
        molecule: Option<Molecule>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::claim_shadow_wallet::{MutationClaimShadowWallet, ClaimShadowWalletParams};
        use crate::mutation::Mutation;

        self.log("info", &format!("KnishIOClient::claim_shadow_wallet() - Claiming shadow wallet for token: {}...", token));

        // Create or use provided molecule (matches JS line 541: const _molecule = molecule || await this.createMolecule({}))
        let mol = match molecule {
            Some(m) => m,
            None => self.create_molecule(None, None, None, None).await?
        };

        // Create mutation (matches JS line 543: const mutation = new mutationClass(this.client(), this, _molecule))
        let mut mutation = MutationClaimShadowWallet::from_molecule(mol);

        // Fill molecule with token and batchId (matches JS lines 1582-1585: query.fillMolecule({ token, batchId }))
        let params = ClaimShadowWalletParams {
            token: token.to_string(),
            batch_id: batch_id.map(|s| s.to_string()),
        };

        // Create a wallet for fill_molecule (shadow wallet claims use a temporary wallet)
        let secret = self.secret.as_deref().ok_or(KnishIOError::MissingSecret)?;
        let bundle = self.bundle.as_deref();
        let wallet = Wallet::create(Some(secret), bundle, token, None, None)?;

        mutation.fill_molecule(params, &wallet)?;

        // Execute mutation (matches JS line 1587: return await this.executeQuery(query))
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Claim all shadow wallets for a token (equivalent to claimShadowWallets in JS)
    ///
    /// Matches JS claimShadowWallets({ token }) at lines 1598-1622
    ///
    /// # Parameters
    /// - `token`: Token slug to claim shadow wallets for
    ///
    /// # Returns
    /// Vector of responses from claiming each shadow wallet
    pub async fn claim_shadow_wallets(&mut self, token: &str) -> Result<Vec<Box<dyn Response>>> {
        self.log("info", &format!("KnishIOClient::claim_shadow_wallets() - Claiming all shadow wallets for token: {}...", token));

        // Query wallets for the token (matches JS line 1602: const shadowWallets = await this.queryWallets({ token }))
        let shadow_wallets = self.query_wallets(None, Some(token)).await?;

        // Validate we got wallets (matches JS lines 1603-1605: if (!shadowWallets || !Array.isArray(shadowWallets)) throw new WalletShadowException())
        if shadow_wallets.is_empty() {
            return Err(KnishIOError::WalletShadow);
        }

        // Validate all wallets are shadow wallets (matches JS lines 1607-1611: shadowWallets.forEach(shadowWallet => { if (!shadowWallet.isShadow()) throw new WalletShadowException() }))
        for wallet in &shadow_wallets {
            if !wallet.is_shadow() {
                return Err(KnishIOError::WalletShadow);
            }
        }

        // Claim each shadow wallet (matches JS lines 1615-1620: for (const shadowWallet of shadowWallets) { responses.push(await this.claimShadowWallet({token, batchId: shadowWallet.batchId})) })
        let mut responses = Vec::new();
        for shadow_wallet in shadow_wallets {
            let batch_id = shadow_wallet.batch_id.as_deref();
            let response = self.claim_shadow_wallet(token, batch_id, None).await?;
            responses.push(response);
        }

        Ok(responses)
    }

    /// Create rule
    ///
    /// Matches JS createRule({ metaType, metaId, rule, policy }) at lines 1219-1245
    ///
    /// # Parameters
    /// - `meta_type`: Type of metadata for the rule
    /// - `meta_id`: ID of metadata for the rule
    /// - `rule`: Rule definition as JSON array (Vec<Value>)
    /// - `policy`: Optional policy as JSON object (HashMap)
    ///
    /// # Returns
    /// Created rule response
    pub async fn create_rule(
        &mut self,
        meta_type: &str,
        meta_id: &str,
        rule: Vec<Value>,
        policy: Option<HashMap<String, Value>>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::create_rule::{MutationCreateRule, CreateRuleParams};
        use crate::mutation::Mutation;

        // Create molecule with secret (matches JS lines 1230-1233)
        let secret = self.secret.as_ref()
            .ok_or(KnishIOError::MissingSecret)?;

        let mut molecule = Molecule::new();
        molecule.secret = Some(secret.clone());

        // Create mutation (matches JS lines 1228-1235)
        let mut mutation = MutationCreateRule::from_molecule(molecule);

        // Fill molecule with rule data (matches JS lines 1237-1242)
        mutation.fill_molecule(CreateRuleParams {
            meta_type: meta_type.to_string(),
            meta_id: meta_id.to_string(),
            rule,
            policy: if let Some(p) = policy {
                serde_json::to_value(p).unwrap_or(Value::Object(serde_json::Map::new()))
            } else {
                Value::Object(serde_json::Map::new())
            },
        })?;

        // Execute mutation (matches JS line 1244)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Create metadata
    ///
    /// Matches JS createMeta({ metaType, metaId, meta, policy }) at lines 1256-1284
    ///
    /// # Parameters
    /// - `meta_type`: Type of metadata
    /// - `meta_id`: ID of metadata
    /// - `meta`: Metadata HashMap
    /// - `policy`: Optional policy HashMap
    ///
    /// # Returns
    /// Created metadata response
    pub async fn create_meta(
        &mut self,
        meta_type: &str,
        meta_id: &str,
        meta: HashMap<String, Value>,
        policy: Option<HashMap<String, Value>>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::create_meta::{MutationCreateMeta, CreateMetaParams};
        use crate::mutation::Mutation;

        // Create molecule with secret and source wallet (matches JS lines 1267-1271)
        let secret = self.secret.as_ref()
            .ok_or(KnishIOError::MissingSecret)?;

        let mut molecule = Molecule::new();
        molecule.secret = Some(secret.clone());

        // For now, we'll skip the complex source_wallet logic and just set the secret
        // The mutation's fill_molecule will handle the rest

        // Create mutation (matches JS lines 1265-1272)
        let mut mutation = MutationCreateMeta::from_molecule(molecule);

        // Fill molecule with metadata (matches JS lines 1276-1281)
        mutation.fill_molecule(CreateMetaParams {
            meta_type: meta_type.to_string(),
            meta_id: meta_id.to_string(),
            meta,
            policy: policy.unwrap_or_default(),
        })?;

        // Execute mutation (matches JS line 1283)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Create identifier
    ///
    /// Matches JS createIdentifier({ type, contact, code }) at lines 1294-1313
    ///
    /// # Parameters
    /// - `identifier_type`: Type of identifier
    /// - `contact`: Contact information
    /// - `code`: Identifier code
    ///
    /// # Returns
    /// Created identifier response
    pub async fn create_identifier(&mut self, identifier_type: &str, contact: &str, code: &str) -> Result<Box<dyn Response>> {
        use crate::mutation::create_identifier::{MutationCreateIdentifier, CreateIdentifierParams};
        use crate::mutation::Mutation;

        // Create mutation (matches JS lines 1302-1304)
        let mut mutation = MutationCreateIdentifier::from_molecule(Molecule::new());

        // Fill molecule with identifier data (matches JS lines 1306-1310)
        mutation.fill_molecule(CreateIdentifierParams {
            r#type: identifier_type.to_string(),
            contact: contact.to_string(),
            code: code.to_string(),
        })?;

        // Execute mutation (matches JS line 1312)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Link an identifier to a wallet bundle
    ///
    /// Matches TS linkIdentifier({ type, contact }) at lines 1731-1763
    ///
    /// # Parameters
    /// - `identifier_type`: Type of identifier
    /// - `contact`: Contact information to link
    ///
    /// # Returns
    /// Link identifier response
    pub async fn link_identifier(&mut self, identifier_type: &str, contact: &str) -> Result<Box<dyn Response>> {
        use crate::mutation::link_identifier::MutationLinkIdentifier;
        use crate::query::Query;

        self.log("info", &format!("KnishIOClient::link_identifier() - Linking identifier of type {}...", identifier_type));

        // Get bundle hash (matches TS line 1743)
        let bundle = self.get_bundle()
            .ok_or(KnishIOError::MissingBundle)?
            .to_string();

        // Create mutation (matches TS line 1740)
        let mutation = MutationLinkIdentifier::new();

        // Build variables (matches TS lines 1746-1750 equivalent)
        let variables = serde_json::json!({
            "bundle": bundle,
            "type": identifier_type,
            "content": contact
        });

        // Execute mutation (matches TS line 1756)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, Some(variables), None).await
    }

    /// Declare an active User Session with a given MetaAsset
    ///
    /// Matches JS activeSession({ bundle, metaType, metaId, ... }) at lines 1111-1135
    ///
    /// # Parameters
    /// - `bundle`: Bundle hash for the session
    /// - `meta_type`: Type of metadata
    /// - `meta_id`: Metadata ID
    /// - `ip_address`: Optional IP address
    /// - `browser`: Optional browser info
    /// - `os_cpu`: Optional OS/CPU info
    /// - `resolution`: Optional screen resolution
    /// - `time_zone`: Optional time zone
    /// - `json`: Additional JSON data
    ///
    /// # Returns
    /// Active session response
    pub async fn active_session(
        &mut self,
        bundle: &str,
        meta_type: &str,
        meta_id: &str,
        ip_address: Option<&str>,
        browser: Option<&str>,
        os_cpu: Option<&str>,
        resolution: Option<&str>,
        time_zone: Option<&str>,
        json: HashMap<String, Value>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::active_session::MutationActiveSession;
        use crate::mutation::Mutation;

        // Create mutation instance (matches JS line 1122)
        let mutation = MutationActiveSession::new();

        // Build variables object (matches JS lines 1124-1134)
        let mut vars = serde_json::Map::new();
        vars.insert("bundleHash".to_string(), Value::String(bundle.to_string()));
        vars.insert("metaType".to_string(), Value::String(meta_type.to_string()));
        vars.insert("metaId".to_string(), Value::String(meta_id.to_string()));

        // Add optional parameters if provided
        if let Some(ip) = ip_address {
            vars.insert("ipAddress".to_string(), Value::String(ip.to_string()));
        }
        if let Some(b) = browser {
            vars.insert("browser".to_string(), Value::String(b.to_string()));
        }
        if let Some(os) = os_cpu {
            vars.insert("osCpu".to_string(), Value::String(os.to_string()));
        }
        if let Some(res) = resolution {
            vars.insert("resolution".to_string(), Value::String(res.to_string()));
        }
        if let Some(tz) = time_zone {
            vars.insert("timeZone".to_string(), Value::String(tz.to_string()));
        }

        // Stringify json parameter (matches JS: json: JSON.stringify(json))
        let json_str = serde_json::to_string(&json)?;
        vars.insert("json".to_string(), Value::String(json_str));

        // Execute mutation (matches JS line 1124: executeQuery)
        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, Some(Value::Object(vars)), None).await
    }

    /// Create policy
    ///
    /// Matches JS createPolicy({ metaType, metaId, policy }) at lines 1324-1349
    ///
    /// # Parameters
    /// - `meta_type`: Type of metadata
    /// - `meta_id`: Metadata ID
    /// - `policy`: Policy definition
    ///
    /// # Returns
    /// Created policy response
    pub async fn create_policy(
        &mut self,
        meta_type: &str,
        meta_id: &str,
        _policy: HashMap<String, Value>
    ) -> Result<Box<dyn Response>> {
        use crate::mutation::propose_molecule::MutationProposeMolecule;
        use crate::mutation::Mutation;

        // Ensure we have authentication (matches JS: client must be authenticated)
        self.ensure_authentication(None).await?;

        // Create molecule with secret and source wallet (matches JS line 1330)
        let secret = self.secret.as_ref()
            .ok_or(KnishIOError::MissingSecret)?;

        let mut molecule = Molecule::new();
        molecule.secret = Some(secret.clone());

        // Get source wallet for the molecule (amount=0.0 since we're just creating a policy atom)
        let source_wallet = self.query_source_wallet("USER", 0.0, None).await?;
        molecule.source_wallet = Some(source_wallet);

        // Add policy atom (matches JS lines 1331-1336)
        // Note: Currently passes empty Vec for meta (matching JS's meta: {})
        // Policy parameter is not yet used by add_policy_atom (TODO in TIER 6.1)
        molecule.add_policy_atom(
            meta_type,
            meta_id,
            Vec::new(), // Empty meta matching JS's meta: {}
            None, // Policy handling is TODO in add_policy_atom
        )?;

        // Add ContinuID atom (matches JS line 1337)
        molecule.add_continuid_atom()?;

        // Sign molecule (matches JS lines 1338-1340)
        let bundle = self.bundle.clone();
        molecule.sign(bundle, false, false)?;

        // Check molecule (matches JS line 1341)
        molecule.check(None)?;

        // Create and execute ProposeMolecule mutation (matches JS lines 1344-1348)
        let mutation = MutationProposeMolecule::from_molecule(molecule);

        let client = self.client.as_ref()
            .ok_or(KnishIOError::NoClient)?;

        mutation.execute(client, None, None).await
    }

    /// Request guest auth token
    ///
    /// # Parameters
    /// - `cell_slug`: Optional cell slug
    ///
    /// # Returns
    /// Guest authentication token
    pub async fn request_guest_auth_token(&mut self, cell_slug: Option<&str>, encrypt: Option<bool>) -> Result<AuthToken> {
        use crate::mutation::request_authorization_guest::MutationRequestAuthorizationGuest;
        use crate::mutation::Mutation;
        use crate::auth::AuthToken;
        use crate::crypto::generate_secret;

        // Set cell slug if provided (matches JS: this.setCellSlug(cellSlug))
        if let Some(slug) = cell_slug {
            self.cell_slug = Some(slug.to_string());
        }

        // Create wallet from fingerprint alternative
        // JS: generateSecret(await this.getFingerprint())
        // Rust: Use fixed seed since we don't have browser fingerprinting
        let secret = generate_secret("guest-default-seed");

        let wallet = Wallet::new(
            Some(&secret),
            None,
            Some("AUTH"),
            None,
            None,
            None,
            None,
        )?;

        // Create mutation
        if let Some(ref client) = self.client.clone() {
            let mutation = MutationRequestAuthorizationGuest::new();

            // Build variables (matches JS: { cellSlug, pubkey: wallet.pubkey, encrypt })
            let mut variables = serde_json::Map::new();

            if let Some(slug) = cell_slug {
                variables.insert("cellSlug".to_string(), serde_json::json!(slug));
            }

            variables.insert("pubkey".to_string(), serde_json::json!(wallet.pubkey));

            if let Some(enc) = encrypt {
                variables.insert("encrypt".to_string(), serde_json::json!(enc));
            }

            // Execute mutation
            let response = mutation.execute(&client, Some(serde_json::Value::Object(variables)), None).await?;

            // Check if successful (matches JS: if (response.success()))
            if response.success() {
                // Extract token data from response payload
                let payload = response.payload()
                    .ok_or(KnishIOError::InvalidResponse)?;

                let token_str = payload.get("token")
                    .and_then(|t| t.as_str())
                    .ok_or(KnishIOError::InvalidResponse)?
                    .to_string();

                let pubkey = payload.get("pubkey")
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string());

                // Parse expires_at from payload
                let expires_at = payload.get("expiresAt")
                    .and_then(|e| e.as_i64());

                let encrypt_setting = payload.get("encrypt")
                    .and_then(|e| e.as_bool());

                // Create AuthToken (matches JS: AuthToken.create(response.payload(), wallet))
                let auth_token = AuthToken::create(
                    token_str,
                    expires_at,
                    encrypt_setting,
                    pubkey,
                    wallet,
                );

                // Set in client (matches JS: this.setAuthToken(authToken))
                self.auth_token = Some(auth_token.clone());

                Ok(auth_token)
            } else {
                let reason = response.reason().unwrap_or_else(|| "Unknown reason".to_string());
                Err(KnishIOError::Custom(format!(
                    "KnishIOClient::request_guest_auth_token() - Authorization attempt rejected by ledger. Reason: {}",
                    reason
                )))
            }
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Request profile auth token (matches JS requestProfileAuthToken)
    ///
    /// # Parameters
    /// - `secret`: User secret for authentication
    /// - `encrypt`: Whether to encrypt the auth token
    ///
    /// # Returns
    /// Profile authentication token
    pub async fn request_profile_auth_token(&mut self, secret: &str, encrypt: Option<bool>) -> Result<AuthToken> {
        use crate::mutation::request_authorization::MutationRequestAuthorization;
        use crate::mutation::Mutation;
        use crate::auth::AuthToken;

        // Set secret in client
        self.secret = Some(secret.to_string());

        // Create AUTH wallet from secret
        let wallet = Wallet::new(
            Some(secret),
            None,
            Some("AUTH"),
            None,
            None,
            None,
            None,
        )?;

        // Create molecule with secret and source wallet
        let mut molecule = Molecule::new();
        molecule.secret = Some(secret.to_string());
        molecule.source_wallet = Some(wallet.clone());

        // Create mutation
        if let Some(ref client) = self.client.clone() {
            let mut mutation = MutationRequestAuthorization::from_molecule(molecule);

            // Fill molecule with encrypt meta (matches JS: fillMolecule({ meta: { encrypt: 'true' } }))
            let mut meta_map = HashMap::new();
            meta_map.insert(
                "encrypt".to_string(),
                serde_json::json!(encrypt.unwrap_or(false).to_string())
            );

            mutation.fill_molecule(crate::mutation::request_authorization::RequestAuthorizationParams {
                meta: meta_map
            })?;

            // Execute mutation
            let response = mutation.execute(&client, None, None).await?;

            // Check if successful
            if response.success() {
                // Extract token from response payload
                let payload = response.payload()
                    .ok_or(KnishIOError::InvalidResponse)?;

                let token_str = payload.get("token")
                    .and_then(|t| t.as_str())
                    .ok_or(KnishIOError::InvalidResponse)?
                    .to_string();

                let expires_at = payload.get("expiresAt")
                    .and_then(|e| e.as_i64());

                let pubkey = payload.get("pubkey")
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string());

                // Create AuthToken (matches JS: AuthToken.create(response.payload(), wallet))
                let auth_token = AuthToken::create(
                    token_str,
                    expires_at,
                    encrypt,
                    pubkey,
                    wallet,
                );

                // Store in self.auth_token
                self.auth_token = Some(auth_token.clone());

                Ok(auth_token)
            } else {
                let reason = response.reason().unwrap_or_else(|| "Unknown reason".to_string());
                Err(KnishIOError::Custom(format!(
                    "KnishIOClient::request_profile_auth_token() - Authorization attempt rejected by ledger. Reason: {}",
                    reason
                )))
            }
        } else {
            Err(KnishIOError::NoClient)
        }
    }

    /// Request auth token
    ///
    /// # Parameters
    /// - `secret`: Optional secret (uses client secret if None)
    /// - `cell_slug`: Optional cell slug
    ///
    /// # Returns
    /// Authentication token
    /// Request auth token with dual-path (profile or guest) authentication
    ///
    /// Matches JS requestAuthToken({ secret, seed, cellSlug, encrypt })
    ///
    /// # Parameters
    /// - `secret`: Optional user secret for profile auth
    /// - `seed`: Optional seed to generate secret from
    /// - `cell_slug`: Optional cell slug for guest auth
    /// - `encrypt`: Optional encryption setting
    ///
    /// # Returns
    /// Authentication token (profile or guest)
    pub async fn request_auth_token(
        &mut self,
        secret: Option<&str>,
        seed: Option<&str>,
        cell_slug: Option<&str>,
        encrypt: Option<bool>
    ) -> Result<AuthToken> {
        use crate::crypto::generate_secret;

        // SDK versions 2 and below do not utilize an authorization token (matches JS line 2118-2122)
        if self.server_sdk_version < 3 {
            self.log("warn", "KnishIOClient::request_auth_token() - Server SDK version does not require an authorization...");
            return Err(KnishIOError::Custom("Server SDK version does not require authorization token".to_string()));
        }

        // Generate a secret from the seed if it has been passed (matches JS line 2124-2127)
        let mut working_secret = secret.map(|s| s.to_string());
        if working_secret.is_none() {
            if let Some(s) = seed {
                working_secret = Some(generate_secret(s));
            }
        }

        // Set cell slug if it has been passed (matches JS line 2129-2132)
        if let Some(slug) = cell_slug {
            self.cell_slug = Some(slug.to_string());
        }

        // Auth in process (matches JS line 2135) — must be reset on ALL exit paths below
        self.auth_in_process = true;

        // Inner block captures Result so we can always reset the flag
        let result: Result<AuthToken> = async {
            // Dual-path authentication (matches JS line 2140-2152)
            let auth_token = if let Some(ref sec) = working_secret {
                // Authorized user - use profile auth
                self.request_profile_auth_token(sec, encrypt).await?
            } else {
                // Guest - use guest auth
                self.request_guest_auth_token(cell_slug, encrypt).await?
            };

            // Log success (matches JS line 2155)
            self.log("info", &format!(
                "KnishIOClient::request_auth_token() - Successfully retrieved auth token {}...",
                auth_token.get_token()
            ));

            // Switch encryption mode if it has been changed (matches JS line 2158)
            self.switch_encryption(encrypt.unwrap_or(false));

            Ok(auth_token)
        }.await;

        // Always reset flag, regardless of success or failure (matches JS line 2161)
        self.auth_in_process = false;
        result
    }

    /// Switch encryption mode
    ///
    /// Matches JS switchEncryption(encrypt)
    ///
    /// # Parameters
    /// - `encrypt`: Whether to enable encryption
    ///
    /// # Returns
    /// true if encryption mode was changed, false if already set
    pub fn switch_encryption(&mut self, encrypt: bool) -> bool {
        // Check if encrypt is already set to that value (matches JS line 204-206)
        if self.encrypt == encrypt {
            return false;
        }

        // Log the change (matches JS line 207)
        self.log("info", &format!(
            "KnishIOClient::switch_encryption() - Forcing encryption {} to match node...",
            if encrypt { "on" } else { "off" }
        ));

        // Set encryption (matches JS line 210)
        self.encrypt = encrypt;

        // Set encryption on GraphQL client (matches JS line 211)
        if let Some(ref mut client) = self.client {
            client.set_encryption(encrypt);
        }

        true
    }
}

/// URI parameter enum to support both single URI and multiple URIs
pub enum UriParam {
    Single(String),
    Multiple(Vec<String>),
}

impl From<String> for UriParam {
    fn from(uri: String) -> Self {
        UriParam::Single(uri)
    }
}

impl From<&str> for UriParam {
    fn from(uri: &str) -> Self {
        UriParam::Single(uri.to_string())
    }
}

impl From<Vec<String>> for UriParam {
    fn from(uris: Vec<String>) -> Self {
        UriParam::Multiple(uris)
    }
}

impl From<Vec<&str>> for UriParam {
    fn from(uris: Vec<&str>) -> Self {
        UriParam::Multiple(uris.into_iter().map(|s| s.to_string()).collect())
    }
}

// Include all the parameter structs and trait definitions from the original file...
// (These remain unchanged)

// Implement Clone for KnishIOClient (required for authentication methods)
impl Clone for KnishIOClient {
    fn clone(&self) -> Self {
        KnishIOClient {
            uris: self.uris.clone(),
            current_uri_index: self.current_uri_index,
            cell_slug: self.cell_slug.clone(),
            secret: self.secret.clone(),
            bundle: self.bundle.clone(),
            auth_token: self.auth_token.clone(),
            auth_token_objects: self.auth_token_objects.clone(),
            auth_in_process: self.auth_in_process,
            server_sdk_version: self.server_sdk_version,
            encrypt: self.encrypt,
            logging: self.logging,
            client: self.client.clone(),
            socket_config: self.socket_config.clone(),
            websocket_client: None, // Don't clone websocket client
            subscription_manager: self.subscription_manager.clone(),
            remainder_wallet: self.remainder_wallet.clone(),
            last_molecule_query: self.last_molecule_query.clone(),
            abort_controllers: Arc::new(Mutex::new(HashMap::new())), // Create new Arc for clone
        }
    }
}

// Implement Debug for KnishIOClient (required for some operations)
impl std::fmt::Debug for KnishIOClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnishIOClient")
            .field("uris", &self.uris)
            .field("cell_slug", &self.cell_slug)
            .field("has_secret", &self.has_secret())
            .field("has_bundle", &self.has_bundle())
            .field("server_sdk_version", &self.server_sdk_version)
            .field("encrypt", &self.encrypt)
            .field("logging", &self.logging)
            .finish()
    }
}