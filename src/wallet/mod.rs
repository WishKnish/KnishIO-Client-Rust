//! Wallet module for the KnishIO SDK
//!
//! This module provides the Wallet struct and associated methods for wallet
//! management, ensuring exact compatibility with the JavaScript implementation.

use crate::crypto::{generate_address, generate_bundle_hash, generate_key};
use crate::error::{KnishIOError, Result};
use crate::types::TokenUnit;
use base64::Engine as _;
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// ML-KEM parameter set for post-quantum key encapsulation
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MlKemParameterSet {
    /// ML-KEM-1024 (FIPS 203 Category 5) - CNSA 2.0 compliant (default)
    #[default]
    MlKem1024,
    /// ML-KEM-768 (FIPS 203 Category 3) - Optional step-back
    MlKem768,
}

impl MlKemParameterSet {
    pub const fn pk_bytes(&self) -> usize {
        match self {
            Self::MlKem1024 => 1568,
            Self::MlKem768 => 1184,
        }
    }

    pub const fn sk_bytes(&self) -> usize {
        match self {
            Self::MlKem1024 => 3168,
            Self::MlKem768 => 2400,
        }
    }

    pub const fn ct_bytes(&self) -> usize {
        match self {
            Self::MlKem1024 => 1568,
            Self::MlKem768 => 1088,
        }
    }
}

/// Wallet structure representing cryptographic keys and token management
///
/// The Wallet struct maintains exact compatibility with the JavaScript implementation,
/// including shadow wallet support, ML-KEM quantum encryption, and token unit management.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    /// Token slug this wallet is intended for (e.g., "USER", "TEST")
    pub token: String,
    
    /// Current balance of the wallet (stored as String for arbitrary-precision integers)
    #[serde(deserialize_with = "deserialize_balance")]
    pub balance: String,
    
    /// Wallet address (hexadecimal public key)
    pub address: Option<String>,
    
    /// Position string used to salt the secret for one-time signatures
    pub position: Option<String>,
    
    /// Bundle hash - 64-character hexadecimal user identifier
    pub bundle: Option<String>,
    
    /// Batch ID for grouped transactions
    #[serde(rename = "batchId")]
    pub batch_id: Option<String>,
    
    /// Character encoding for signatures
    pub characters: Option<String>,
    
    /// Private key for signing (4096 characters)
    #[serde(skip_serializing)]
    pub key: Option<String>,
    
    /// ML-KEM public key for quantum encryption
    pub pubkey: Option<String>,
    
    /// ML-KEM private key for quantum decryption
    #[serde(skip_serializing)]
    pub privkey: Option<Vec<u8>>,
    
    /// Token units owned by this wallet
    #[serde(rename = "tokenUnits")]
    pub token_units: Vec<TokenUnit>,
    
    /// Trade rates for buffer operations
    #[serde(rename = "tradeRates")]
    pub trade_rates: HashMap<String, f64>,
    
    /// Molecules associated with this wallet
    pub molecules: HashMap<String, serde_json::Value>,
    /// ML-KEM parameter set (default: ML-KEM-1024)
    #[serde(default)]
    pub mlkem_parameter_set: MlKemParameterSet,
}

/// Debug impl that redacts sensitive cryptographic material (private key, ML-KEM private key)
impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("token", &self.token)
            .field("balance", &self.balance)
            .field("address", &self.address)
            .field("position", &self.position)
            .field("bundle", &self.bundle)
            .field("batch_id", &self.batch_id)
            .field("characters", &self.characters)
            .field("key", &self.key.as_ref().map(|_| "[REDACTED]"))
            .field("pubkey", &self.pubkey)
            .field("privkey", &self.privkey.as_ref().map(|_| "[REDACTED]"))
            .field("token_units", &self.token_units)
            .field("trade_rates", &self.trade_rates)
            .field("molecules", &self.molecules)
            .finish()
    }
}

impl Wallet {
    /// Create a new Wallet instance
    ///
    /// # Arguments
    /// 
    /// * `secret` - 2048-character biometric hash (optional for shadow wallets)
    /// * `bundle` - 64-character hexadecimal user identifier (optional)
    /// * `token` - Token slug (defaults to "USER")
    /// * `address` - Hexadecimal public key (optional)
    /// * `position` - Position string (optional)
    /// * `batch_id` - Batch ID for transactions (optional)
    /// * `characters` - Character encoding (optional)
    /// * `mlkem_parameter_set` - ML-KEM parameter set (optional; defaults to ML-KEM-1024)
    pub fn new(
        secret: Option<&str>,
        bundle: Option<&str>,
        token: Option<&str>,
        address: Option<&str>,
        position: Option<&str>,
        batch_id: Option<&str>,
        characters: Option<&str>,
        mlkem_parameter_set: Option<MlKemParameterSet>,
    ) -> Result<Self> {
        let token = token.unwrap_or("USER").to_string();
        
        let mut wallet = Wallet {
            token: token.clone(),
            balance: "0".to_string(),
            address: address.map(|s| s.to_string()),
            position: position.map(|s| s.to_string()),
            bundle: bundle.map(|s| s.to_string()),
            batch_id: batch_id.map(|s| s.to_string()),
            characters: characters.map(|s| s.to_string()),
            key: None,
            pubkey: None,
            privkey: None,
            token_units: Vec::new(),
            trade_rates: HashMap::new(),
            molecules: HashMap::new(),
            mlkem_parameter_set: mlkem_parameter_set.unwrap_or_default(),
        };

        if let Some(secret) = secret {
            // Set bundle from the secret if not provided
            if wallet.bundle.is_none() {
                wallet.bundle = Some(generate_bundle_hash(secret));
            }

            // Generate position for non-shadow wallet if not initialized
            if wallet.position.is_none() {
                wallet.position = Some(Self::generate_position(64));
            }

            // Key & address initialization
            if let Some(position) = &wallet.position {
                wallet.key = Some(generate_key(secret, &token, position));
                
                if wallet.address.is_none() {
                    if let Some(key) = &wallet.key {
                        wallet.address = Some(generate_address(key)?);
                    }
                }
            }

            // Set default characters
            if wallet.characters.is_none() {
                wallet.characters = Some("BASE64".to_string());
            }

            // Initialize ML-KEM keys
            wallet.initialize_mlkem()?;
        }

        Ok(wallet)
    }

    /// Create a new Wallet instance using the builder pattern
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret string (optional)
    /// * `bundle` - Bundle hash (optional)
    /// * `token` - Token slug
    /// * `position` - Position string (optional, generated if not provided)
    /// * `characters` - Character encoding (optional)
    /// * `mlkem_parameter_set` - ML-KEM parameter set (optional; defaults to ML-KEM-1024)
    pub fn create(
        secret: Option<&str>,
        bundle: Option<&str>,
        token: &str,
        position: Option<&str>,
        characters: Option<&str>,
        mlkem_parameter_set: Option<MlKemParameterSet>,
    ) -> Result<Self> {
        // Validate credentials
        if secret.is_none() && bundle.is_none() {
            return Err(KnishIOError::WalletCredential);
        }

        let mut final_position = position.map(|s| s.to_string());
        let mut final_bundle = bundle.map(|s| s.to_string());

        // Generate position and bundle if secret provided but no bundle
        if secret.is_some() && bundle.is_none() {
            // Only generate position if not provided
            if final_position.is_none() {
                final_position = Some(Self::generate_position(64));
            } 
            if let Some(secret) = secret {
                final_bundle = Some(generate_bundle_hash(secret));
            }
        }

        Self::new(
            secret,
            final_bundle.as_deref(),
            Some(token),
            None,
            final_position.as_deref(),
            None,  // batch_id
            characters,
            mlkem_parameter_set,
        )
    }

    /// Create wallet from GraphQL response data (matches JS implementation)
    ///
    /// # Arguments
    ///
    /// * `data` - Response data from GraphQL query
    ///
    /// # Returns
    ///
    /// Result containing the wallet instance
    pub fn from_response_data(data: serde_json::Value) -> Result<Self> {
        // The Balance query selects `amount` (the validator's balance field); fall back to
        // `balance` for other shapes. Reading only `balance` (absent from the Balance selection
        // set) silently yielded 0 for every live balance query.
        let balance_val = if !data["amount"].is_null() { &data["amount"] } else { &data["balance"] };
        // Use pattern matching for cleaner data extraction
        let (balance, token, address, bundle, position, characters, batch_id) = (
            // Accept balance as string, number, or integer from JSON
            match balance_val {
                v if v.is_string() => v.as_str().unwrap_or("0").to_string(),
                v if v.is_number() => {
                    // Prefer i64 to avoid f64 precision loss, fallback to f64 for decimals
                    if let Some(i) = v.as_i64() {
                        i.to_string()
                    } else {
                        format!("{}", v.as_f64().unwrap_or(0.0) as i128)
                    }
                }
                _ => "0".to_string(),
            },
            data["tokenSlug"].as_str().unwrap_or("USER"),
            data["address"].as_str(),
            data["bundleHash"].as_str(), 
            data["position"].as_str(),
            data["characters"].as_str(),
            data["batchId"].as_str(),
        );

        let mut wallet = Self::new(
            None, // No secret when creating from response data
            bundle,
            Some(token),
            address,
            position,
            batch_id,
            characters,
            None,
        )?;
        wallet.balance = balance;

        // Parse token units. The GraphQL Balance response returns each unit as an OBJECT
        // { id, name, metas } (metas is a String scalar / null on the wire), so parse the object
        // form; tolerate the array-of-arrays wire form [id, name, metas] too (the atom-meta shape).
        // Without the object branch, query_balance returned empty token_units, silently degrading
        // every stackable transfer to fungible (units never moved). The unit id is what matters.
        wallet.token_units = data["tokenUnits"]
            .as_array()
            .map(|units_data| {
                units_data
                    .iter()
                    .filter_map(|unit_data| {
                        if let Some(obj) = unit_data.as_object() {
                            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if id.is_empty() {
                                return None;
                            }
                            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let meta = obj.get("metas")
                                .and_then(|v| v.as_object())
                                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
                            Some(TokenUnit::new(id, name, meta))
                        } else if let Some(unit_array) = unit_data.as_array() {
                            if unit_array.len() < 2 {
                                return None;
                            }
                            let id = unit_array[0].as_str().unwrap_or("").to_string();
                            let name = unit_array[1].as_str().map(|s| s.to_string()).unwrap_or_default();
                            let meta = (unit_array.len() > 2)
                                .then(|| unit_array[2].as_object())
                                .flatten()
                                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
                            Some(TokenUnit::new(id, name, meta))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(wallet)
    }

    /// Set wallet keys from secret (matches JS setKeyFromSecret behavior)
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret string
    /// * `token` - Token slug 
    /// * `position` - Position string
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn set_key_from_secret(&mut self, secret: &str, token: &str, position: &str) -> Result<()> {
        self.token = token.to_string();
        self.position = Some(position.to_string());
        
        // Generate bundle hash if not present
        if self.bundle.is_none() {
            self.bundle = Some(generate_bundle_hash(secret));
        }
        
        // Generate key and address
        self.key = Some(generate_key(secret, token, position));
        
        if let Some(key) = &self.key {
            self.address = Some(generate_address(key)?);
        }
        
        // Set default characters
        if self.characters.is_none() {
            self.characters = Some("BASE64".to_string());
        }
        
        // Initialize ML-KEM keys
        self.initialize_mlkem()?;
        
        Ok(())
    }

    /// Initialize batch ID from source wallet (matches JS initBatchIdFromSource)
    ///
    /// # Arguments
    ///
    /// * `source` - Source wallet to copy batch ID from
    pub fn init_batch_id_from_source(&mut self, source: &Wallet) {
        if let Some(ref source_batch_id) = source.batch_id {
            self.batch_id = Some(source_batch_id.clone());
        }
    }

    /// Determine if the provided string is a bundle hash
    ///
    /// # Arguments
    ///
    /// * `maybe_bundle_hash` - String to check
    ///
    /// # Returns
    ///
    /// True if the string is a valid bundle hash
    pub fn is_bundle_hash(maybe_bundle_hash: &str) -> bool {
        maybe_bundle_hash.len() == 64 && maybe_bundle_hash.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Generate a cryptographic key for wallet operations
    ///
    /// Delegates to the crypto module's implementation.
    ///
    /// # Arguments
    ///
    /// * `secret` - The wallet secret
    /// * `token` - The token slug
    /// * `position` - The wallet position
    ///
    /// # Returns
    ///
    /// A 2048-character hexadecimal key string
    pub fn generate_key(secret: &str, token: &str, position: &str) -> String {
        crate::crypto::generate_key(secret, token, position)
    }

    /// Generate a wallet address from a key
    ///
    /// Delegates to the crypto module's implementation.
    ///
    /// # Arguments
    ///
    /// * `key` - The cryptographic key (2048 characters)
    ///
    /// # Returns
    ///
    /// A base17-encoded wallet address
    pub fn generate_address(key: &str) -> Result<String> {
        crate::crypto::generate_address(key)
    }

    /// Generate a random position string
    ///
    /// Creates a random hexadecimal position.
    ///
    /// # Arguments
    ///
    /// * `salt_length` - Length of the position string
    ///
    /// # Returns
    ///
    /// A hexadecimal position string
    pub fn generate_position(salt_length: usize) -> String {
        use rand::RngExt;
        
        const HEX_CHARSET: &[u8] = b"abcdef0123456789";
        
        let mut rng = rand::rng();
        
        // Use iterator with random sampling for better performance
        (0..salt_length)
            .map(|_| HEX_CHARSET[rng.random_range(0..HEX_CHARSET.len())] as char)
            .collect()
    }

    /// Validate that a position string is a valid 64-character hex string
    ///
    /// Positions in the KnishIO protocol are 64-character lowercase hex strings
    /// generated by `generate_position()`. This validates externally-provided positions.
    pub fn is_valid_position(position: &str) -> bool {
        position.len() == 64 && position.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Get formatted token units from raw data
    ///
    /// # Arguments
    ///
    /// * `units_data` - Raw token unit data
    ///
    /// # Returns
    ///
    /// Vector of TokenUnit objects
    pub fn get_token_units(units_data: &[Vec<serde_json::Value>]) -> Vec<TokenUnit> {
        units_data
            .iter()
            .filter(|unit_data| unit_data.len() >= 2)
            .map(|unit_data| {
                let id = unit_data[0].as_str().unwrap_or("").to_string();
                let name = unit_data[1].as_str().map(|s| s.to_string()).unwrap_or_default();
                
                // Use conditional then() for cleaner meta parsing
                let meta = (unit_data.len() > 2)
                    .then(|| unit_data[2].as_object())
                    .flatten()
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
                
                TokenUnit::new(id, name, meta)
            })
            .collect()
    }

    /// Get token units data for serialization
    ///
    /// # Returns
    ///
    /// Vector of token unit data arrays
    pub fn get_token_units_data(&self) -> Vec<Vec<serde_json::Value>> {
        self.token_units.iter().map(|unit| {
            unit.to_data()
        }).collect()
    }

    /// Split token units between wallets
    ///
    /// # Arguments
    ///
    /// * `units` - Token unit IDs to transfer
    /// * `remainder_wallet` - Wallet to receive remaining units
    /// * `recipient_wallet` - Optional wallet to receive specified units
    pub fn split_units(
        &mut self,
        units: &[String],
        remainder_wallet: &mut Wallet,
        recipient_wallet: Option<&mut Wallet>,
    ) {
        if units.is_empty() {
            return;
        }

        // Use partition for cleaner unit splitting
        let (recipient_units, remainder_units): (Vec<_>, Vec<_>) = self
            .token_units
            .iter()
            .cloned()
            .partition(|token_unit| units.contains(&token_unit.id));

        // Update token units using pattern matching
        self.token_units = recipient_units.clone();

        if let Some(recipient) = recipient_wallet {
            recipient.token_units = recipient_units;
        }

        remainder_wallet.token_units = remainder_units;
    }

    /// Split token units across MULTIPLE recipients (WP line 544).
    ///
    /// N-way sibling of `split_units`: the source retains the SENT union (all units leaving),
    /// each recipient gets its own subset, and the remainder gets the KEPT units (those not
    /// assigned to any recipient). `recipient_unit_lists` is parallel to `recipient_wallets`.
    pub fn split_units_multi(
        &mut self,
        recipient_unit_lists: &[Vec<String>],
        recipient_wallets: &mut [Wallet],
        remainder_wallet: &mut Wallet,
    ) {
        use std::collections::HashSet;

        // The union of all unit ids leaving the source
        let sent_ids: HashSet<String> = recipient_unit_lists.iter().flatten().cloned().collect();

        // Nothing to split (fungible transfer) — leave token units untouched
        if sent_ids.is_empty() {
            return;
        }

        // Each recipient gets its own subset of the source's token units
        for (recipient, ids) in recipient_wallets.iter_mut().zip(recipient_unit_lists.iter()) {
            recipient.token_units = self
                .token_units
                .iter()
                .filter(|token_unit| ids.contains(&token_unit.id))
                .cloned()
                .collect();
        }

        // The remainder keeps everything not sent to any recipient (KEPT)
        remainder_wallet.token_units = self
            .token_units
            .iter()
            .filter(|token_unit| !sent_ids.contains(&token_unit.id))
            .cloned()
            .collect();

        // The source carries the SENT union (the ownership authority the validator reads)
        self.token_units = self
            .token_units
            .iter()
            .filter(|token_unit| sent_ids.contains(&token_unit.id))
            .cloned()
            .collect();
    }

    /// Create a remainder wallet from the source wallet
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret for the new wallet
    ///
    /// # Returns
    ///
    /// A new remainder wallet
    pub fn create_remainder(&self, secret: &str) -> Result<Wallet> {
        let mut remainder_wallet = Self::create(
            Some(secret),
            None,
            &self.token,
            None,
            self.characters.as_deref(),
            Some(self.mlkem_parameter_set),
        )?;
        remainder_wallet.init_batch_id(Some(self), true);
        Ok(remainder_wallet)
    }

    /// Check if this wallet is a shadow wallet
    ///
    /// Shadow wallets have no position or address.
    ///
    /// # Returns
    ///
    /// True if this is a shadow wallet
    pub fn is_shadow(&self) -> bool {
        self.position.is_none() && self.address.is_none()
    }

    /// Initialize batch ID for grouped transactions
    ///
    /// # Arguments
    ///
    /// * `source_wallet` - Source wallet to inherit batch ID from
    /// * `is_remainder` - Whether this is a remainder wallet
    pub fn init_batch_id(&mut self, source_wallet: Option<&Wallet>, is_remainder: bool) {
        if let Some(source) = source_wallet {
            if let Some(source_batch_id) = &source.batch_id {
                if is_remainder {
                    self.batch_id = Some(source_batch_id.clone());
                } else {
                    // Generate new batch ID for non-remainder wallets
                    self.batch_id = Some(Self::generate_batch_id());
                }
            }
        }
    }

    /// Generate a new batch ID
    ///
    /// # Returns
    ///
    /// A new batch ID string
    fn generate_batch_id() -> String {
        use rand::RngExt;
        let mut rng = rand::rng();
        format!("{:016x}", rng.random::<u64>())
    }

    /// ML-KEM parameter set implied by a serialized public key's raw byte length
    ///
    /// FIPS 203's key lengths are disjoint (1568 bytes → ML-KEM-1024, 1184 bytes → ML-KEM-768),
    /// so a stored peer key recovers the parameter set of the session it belongs to without any
    /// wire-format change. Used by [`crate::auth::AuthToken::restore`] to resolve a session
    /// snapshot written before the parameter set was persisted.
    ///
    /// # Arguments
    ///
    /// * `pubkey` - Base64-serialized ML-KEM public key
    ///
    /// # Returns
    ///
    /// The implied parameter set, or `None` when the length matches neither
    pub fn mlkem_parameter_set_from_pubkey(pubkey: &str) -> Option<MlKemParameterSet> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(pubkey)
            .ok()?;

        [MlKemParameterSet::MlKem1024, MlKemParameterSet::MlKem768]
            .into_iter()
            .find(|set| set.pk_bytes() == bytes.len())
    }

    /// Derive an ML-KEM keypair at an arbitrary parameter set from this wallet's key
    ///
    /// The 64-byte `d‖z` seed is derived from the Knish.IO wallet key and takes NO
    /// parameter-set input — only the final keygen call differs — so every wallet can
    /// deterministically reproduce both its ML-KEM-768 and its ML-KEM-1024 identity from
    /// material it already holds. Returns `(pubkey_base64, privkey_bytes)`: nothing is stored
    /// on `self`, so the caller owns the derived private key and releases it in its own scope.
    fn derive_mlkem_keypair(&self, set: MlKemParameterSet) -> Result<(String, Vec<u8>)> {
        let key = self.key.as_ref().ok_or(KnishIOError::DecryptionKey)?;

        // Generate a 64-byte (512-bit) seed from the Knish.IO private key
        // Use deterministic approach matching JavaScript: generateSecret(key, 128) → 128 hex chars = 64 bytes
        use crate::crypto::generate_secret_with_params;
        let seed_hex = generate_secret_with_params(Some(key), 128); // 128 hex chars = 64 bytes

        // Convert hex string to 64-byte seed array
        let mut seed = [0u8; 64];
        for i in 0..64 {
            let hex_chars = &seed_hex[i * 2..i * 2 + 2];
            seed[i] = u8::from_str_radix(hex_chars, 16)
                .map_err(|_| KnishIOError::DecryptionKey)?;
        }

        let (pk_bytes, sk_bytes) = match set {
            MlKemParameterSet::MlKem1024 => {
                use libcrux_ml_kem::mlkem1024;
                let keypair = mlkem1024::generate_key_pair(seed);
                (keypair.pk().as_slice().to_vec(), keypair.sk().as_slice().to_vec())
            }
            MlKemParameterSet::MlKem768 => {
                use libcrux_ml_kem::mlkem768;
                let keypair = mlkem768::generate_key_pair(seed);
                (keypair.pk().as_slice().to_vec(), keypair.sk().as_slice().to_vec())
            }
        };

        // Serialize the public key to match JavaScript base64 format
        Ok((
            base64::engine::general_purpose::STANDARD.encode(&pk_bytes),
            sk_bytes,
        ))
    }

    /// Initialize ML-KEM quantum encryption keys
    ///
    /// Sets up the quantum-resistant encryption key pair at the wallet's configured parameter
    /// set (ML-KEM-1024 by default, ML-KEM-768 as the step-back), from the deterministic
    /// wallet-key seed. Matches the JavaScript implementation.
    fn initialize_mlkem(&mut self) -> Result<()> {
        if self.key.is_some() {
            let (pubkey, privkey) = self.derive_mlkem_keypair(self.mlkem_parameter_set)?;
            self.pubkey = Some(pubkey);
            self.privkey = Some(privkey);
        }

        Ok(())
    }

    /// Encrypt a message using ML-KEM quantum encryption
    ///
    /// # Arguments
    ///
    /// * `message` - The message to encrypt
    /// * `recipient_pubkey` - The recipient's public key (base64 encoded)
    ///
    /// # Returns
    ///
    /// Encrypted message data
    pub async fn encrypt_message(
        &self,
        message: &serde_json::Value,
        recipient_pubkey: &str,
    ) -> Result<EncryptedMessage> {
        // Convert message to JSON string and bytes (matches JavaScript)
        let message_string = serde_json::to_string(message)?;
        let message_bytes = message_string.as_bytes();
        
        // Deserialize recipient public key from base64
        let recipient_pubkey_bytes = base64::engine::general_purpose::STANDARD
            .decode(recipient_pubkey)
            .map_err(|_| KnishIOError::DecryptionKey)?;
            
        // Perform ML-KEM encapsulation to get shared secret
        use libcrux_ml_kem::MlKemPublicKey;

        let expected_pk_bytes = self.mlkem_parameter_set.pk_bytes();
        if recipient_pubkey_bytes.len() != expected_pk_bytes {
            return Err(KnishIOError::DecryptionKey);
        }

        // Generate random bytes for encapsulation
        let mut randomness = [0u8; 32];
        rand::rng().fill_bytes(&mut randomness);

        let (ciphertext_bytes, shared_secret_bytes) = match self.mlkem_parameter_set {
            MlKemParameterSet::MlKem1024 => {
                use libcrux_ml_kem::mlkem1024;
                let mut public_key_array = [0u8; 1568];
                public_key_array.copy_from_slice(&recipient_pubkey_bytes);
                let public_key = MlKemPublicKey::from(public_key_array);
                let (ciphertext, shared_secret) = mlkem1024::encapsulate(&public_key, randomness);
                (ciphertext.as_slice().to_vec(), shared_secret.as_slice().to_vec())
            }
            MlKemParameterSet::MlKem768 => {
                use libcrux_ml_kem::mlkem768;
                let mut public_key_array = [0u8; 1184];
                public_key_array.copy_from_slice(&recipient_pubkey_bytes);
                let public_key = MlKemPublicKey::from(public_key_array);
                let (ciphertext, shared_secret) = mlkem768::encapsulate(&public_key, randomness);
                (ciphertext.as_slice().to_vec(), shared_secret.as_slice().to_vec())
            }
        };

        // Encrypt message using AES-GCM with shared secret
        let encrypted_message_bytes = self.encrypt_with_shared_secret(message_bytes, &shared_secret_bytes).await?;
        // Serialize to base64 (matches JavaScript serialization)
        let cipher_text = base64::engine::general_purpose::STANDARD.encode(&ciphertext_bytes);
        let encrypted_message = base64::engine::general_purpose::STANDARD.encode(encrypted_message_bytes);
        
        Ok(EncryptedMessage {
            cipher_text,
            encrypted_message,
        })
    }

    /// Decrypt a message using ML-KEM quantum decryption
    ///
    /// # Arguments
    ///
    /// * `encrypted_data` - The encrypted message data
    ///
    /// # Returns
    ///
    /// Decrypted message
    pub async fn decrypt_message(
        &self,
        encrypted_data: &EncryptedMessage,
    ) -> Result<serde_json::Value> {
        let decrypted_string = self.mlkem_decrypt_to_string(encrypted_data).await?;

        serde_json::from_str::<serde_json::Value>(&decrypted_string)
            .map_err(|_| KnishIOError::DecryptionKey)
    }

    /// ML-KEM decapsulate + AES-256-GCM decrypt → the RAW decrypted UTF-8 string
    ///
    /// Shared by [`Wallet::decrypt_message`] (which JSON-parses the result) and the
    /// post-quantum `CipherHash` map path ([`Wallet::decrypt_my_message_ml`], which needs the
    /// raw response JSON text).
    ///
    /// Inbound is PERMISSIVE: a ciphertext at either ML-KEM parameter set decrypts, provided it
    /// is addressed to one of THIS wallet's own identities. The 64-byte seed is
    /// parameter-set-independent, so the other identity is derived on demand and its private key
    /// is zeroized and dropped inside this call — never cached on the wallet. Outbound
    /// encapsulation stays STRICT (see [`Wallet::encrypt_message`]): reading a 768 record we own
    /// downgrades nothing, but encapsulating at 768 would.
    ///
    /// # Arguments
    ///
    /// * `encrypted_data` - The encrypted message data
    ///
    /// # Returns
    ///
    /// The decrypted plaintext string
    pub async fn mlkem_decrypt_to_string(
        &self,
        encrypted_data: &EncryptedMessage,
    ) -> Result<String> {
        use zeroize::Zeroize;

        // Get private key
        let privkey = self.privkey.as_ref()
            .ok_or(KnishIOError::DecryptionKey)?;

        // Deserialize ciphertext from base64
        let ciphertext_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encrypted_data.cipher_text)
            .map_err(|_| KnishIOError::DecryptionKey)?;

        let configured = self.mlkem_parameter_set;
        let other = match configured {
            MlKemParameterSet::MlKem1024 => MlKemParameterSet::MlKem768,
            MlKemParameterSet::MlKem768 => MlKemParameterSet::MlKem1024,
        };

        // Dispatch decapsulation on the ciphertext's decoded length.
        let shared_secret_bytes = if ciphertext_bytes.len() == configured.ct_bytes() {
            Self::decapsulate(configured, privkey, &ciphertext_bytes)?
        } else if ciphertext_bytes.len() == other.ct_bytes() {
            let (_derived_pubkey, mut derived_privkey) = self.derive_mlkem_keypair(other)?;
            let shared_secret = Self::decapsulate(other, &derived_privkey, &ciphertext_bytes);
            derived_privkey.zeroize();
            shared_secret?
        } else {
            return Err(KnishIOError::DecryptionKey);
        };

        // Deserialize encrypted message from base64
        let encrypted_message_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encrypted_data.encrypted_message)
            .map_err(|_| KnishIOError::DecryptionKey)?;

        // Decrypt message using AES-GCM with shared secret
        let decrypted_bytes = self.decrypt_with_shared_secret(&encrypted_message_bytes, &shared_secret_bytes).await?;

        String::from_utf8(decrypted_bytes)
            .map_err(|_| KnishIOError::DecryptionKey)
    }

    /// ML-KEM decapsulation at an explicit parameter set
    ///
    /// Rejects a private key or ciphertext whose length does not match `set` — the same guard
    /// the single-parameter-set path applied inline, now expressed once for both identities.
    fn decapsulate(
        set: MlKemParameterSet,
        privkey: &[u8],
        ciphertext_bytes: &[u8],
    ) -> Result<Vec<u8>> {
        if privkey.len() != set.sk_bytes() || ciphertext_bytes.len() != set.ct_bytes() {
            return Err(KnishIOError::DecryptionKey);
        }

        use libcrux_ml_kem::{MlKemCiphertext, MlKemPrivateKey};

        Ok(match set {
            MlKemParameterSet::MlKem1024 => {
                use libcrux_ml_kem::mlkem1024;
                let mut secret_key_array = [0u8; 3168];
                secret_key_array.copy_from_slice(privkey);
                let secret_key = MlKemPrivateKey::from(secret_key_array);

                let mut ciphertext_array = [0u8; 1568];
                ciphertext_array.copy_from_slice(ciphertext_bytes);
                let ciphertext = MlKemCiphertext::from(ciphertext_array);

                mlkem1024::decapsulate(&secret_key, &ciphertext).as_slice().to_vec()
            }
            MlKemParameterSet::MlKem768 => {
                use libcrux_ml_kem::mlkem768;
                let mut secret_key_array = [0u8; 2400];
                secret_key_array.copy_from_slice(privkey);
                let secret_key = MlKemPrivateKey::from(secret_key_array);

                let mut ciphertext_array = [0u8; 1088];
                ciphertext_array.copy_from_slice(ciphertext_bytes);
                let ciphertext = MlKemCiphertext::from(ciphertext_array);

                mlkem768::decapsulate(&secret_key, &ciphertext).as_slice().to_vec()
            }
        })
    }

    /// Multi-recipient `CipherHash` map key for a public key
    ///
    /// `Base64_standard(SHAKE256(pubkey_utf8, 8 bytes))` — matches the Rust validator's
    /// `hash_share` and the other SDKs' `hashShare`/`shortHash`.
    ///
    /// # Arguments
    ///
    /// * `pubkey` - Base64-serialized ML-KEM public key
    ///
    /// # Returns
    ///
    /// The map key addressing an envelope to that public key
    pub fn hash_share(pubkey: &str) -> String {
        let digest_hex = crate::crypto::shake256(pubkey, 64);
        let digest = hex::decode(&digest_hex).unwrap_or_default();

        base64::engine::general_purpose::STANDARD.encode(digest)
    }

    /// Decrypt a `CipherHash` map addressed to THIS wallet's ML-KEM public key
    ///
    /// Returns the RAW decrypted text (not JSON-parsed; it replaces the response body for the
    /// normal parser), or `None` when the map holds no entry for this wallet or decryption
    /// fails.
    ///
    /// The configured parameter set's hash share is tried first, then the share of the other
    /// set's public key, derived on demand from this wallet's own key seed: a pre-bump sender
    /// addressed its envelope to `hashShare(our_768_pubkey)`, which a wallet configured at
    /// ML-KEM-1024 would otherwise never find. Only the derived PUBLIC key is used here, and
    /// the derived private key is zeroized before the lookup.
    ///
    /// # Arguments
    ///
    /// * `map` - `hashShare(recipientPubkey)` → envelope map
    ///
    /// # Returns
    ///
    /// The raw decrypted text, if an entry for this wallet decrypts
    pub async fn decrypt_my_message_ml(
        &self,
        map: &HashMap<String, EncryptedMessage>,
    ) -> Option<String> {
        use zeroize::Zeroize;

        let mut envelope = self
            .pubkey
            .as_deref()
            .and_then(|pubkey| map.get(&Self::hash_share(pubkey)));

        if envelope.is_none() {
            let other = match self.mlkem_parameter_set {
                MlKemParameterSet::MlKem1024 => MlKemParameterSet::MlKem768,
                MlKemParameterSet::MlKem768 => MlKemParameterSet::MlKem1024,
            };
            if let Ok((other_pubkey, mut other_privkey)) = self.derive_mlkem_keypair(other) {
                other_privkey.zeroize();
                envelope = map.get(&Self::hash_share(&other_pubkey));
            }
        }

        self.mlkem_decrypt_to_string(envelope?).await.ok()
    }

    /// Encrypt data with AES-256-GCM using shared secret
    async fn encrypt_with_shared_secret(&self, message: &[u8], shared_secret: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use aes_gcm::aead::Aead;
        
        // Use shared secret as AES key
        let cipher = Aes256Gcm::new_from_slice(shared_secret)
            .map_err(|_| KnishIOError::DecryptionKey)?;
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);
        
        // Encrypt message
        let mut encrypted = cipher.encrypt(&nonce, message)
            .map_err(|_| KnishIOError::EncryptionError)?;
        
        // Prepend nonce to encrypted data
        let mut result = nonce_bytes.to_vec();
        result.append(&mut encrypted);
        
        Ok(result)
    }
    
    /// Decrypt data with AES-256-GCM using shared secret
    async fn decrypt_with_shared_secret(&self, encrypted_data: &[u8], shared_secret: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use aes_gcm::aead::Aead;
        
        if encrypted_data.len() < 12 {
            return Err(KnishIOError::DecryptionKey);
        }
        
        // Use shared secret as AES key
        let cipher = Aes256Gcm::new_from_slice(shared_secret)
            .map_err(|_| KnishIOError::DecryptionKey)?;
        
        // Extract nonce and encrypted message
        let nonce = Nonce::try_from(&encrypted_data[..12])
            .map_err(|_| KnishIOError::DecryptionKey)?;
        let encrypted_message = &encrypted_data[12..];
        
        // Decrypt message
        let decrypted = cipher.decrypt(&nonce, encrypted_message)
            .map_err(|_| KnishIOError::DecryptionKey)?;
        
        Ok(decrypted)
    }
}

/// Custom deserializer for balance field that accepts both String and Number JSON values
fn deserialize_balance<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => {
            // Prefer i64 to avoid f64 precision loss
            if let Some(i) = n.as_i64() {
                Ok(i.to_string())
            } else if let Some(f) = n.as_f64() {
                Ok(format!("{}", f as i128))
            } else {
                Ok("0".to_string())
            }
        }
        serde_json::Value::Null => Ok("0".to_string()),
        _ => Ok("0".to_string()),
    }
}

/// Encrypted message structure for quantum encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessage {
    pub cipher_text: String,
    pub encrypted_message: String,
}

// Balance helper methods for precision-safe arithmetic
impl Wallet {
    /// Parse balance as i128 for arithmetic (0 if unparseable)
    pub fn balance_as_i128(&self) -> i128 {
        self.balance.parse::<i128>().unwrap_or_else(|_| {
            // Fallback: try parsing as f64 and converting (for "100.0" style strings)
            self.balance.parse::<f64>().map(|f| f as i128).unwrap_or(0)
        })
    }

    /// Set balance from i128
    pub fn set_balance_i128(&mut self, val: i128) {
        self.balance = val.to_string();
    }

    /// Set balance from f64 (convenience for small amounts)
    pub fn set_balance_f64(&mut self, val: f64) {
        self.balance = format!("{}", val as i128);
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Wallet {
            token: "USER".to_string(),
            balance: "0".to_string(),
            address: None,
            position: None,
            bundle: None,
            batch_id: None,
            characters: None,
            key: None,
            pubkey: None,
            privkey: None,
            token_units: Vec::new(),
            trade_rates: HashMap::new(),
            molecules: HashMap::new(),
            mlkem_parameter_set: MlKemParameterSet::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::create(
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
            None,
            "TEST",
            None,
            None,
            None,
        ).expect("Wallet creation should succeed with valid parameters");
        
        assert_eq!(wallet.token, "TEST");
        assert!(wallet.bundle.is_some());
        assert!(wallet.position.is_some());
        assert!(wallet.address.is_some());
        assert!(wallet.key.is_some());
    }

    #[test]
    fn test_shadow_wallet() {
        let wallet = Wallet::new(
            None,
            Some("test-bundle"),
            Some("TEST"),
            None,
            None,
            None,
            None,
            None,
        ).unwrap();
        
        assert!(wallet.is_shadow());
        assert_eq!(wallet.token, "TEST");
        assert_eq!(wallet.bundle, Some("test-bundle".to_string()));
    }

    #[test]
    fn test_bundle_hash_validation() {
        assert!(Wallet::is_bundle_hash("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
        assert!(!Wallet::is_bundle_hash("invalid"));
        assert!(!Wallet::is_bundle_hash("0123456789abcdef")); // too short
    }

    #[test]
    fn test_position_generation() {
        let pos1 = Wallet::generate_position(64);
        let pos2 = Wallet::generate_position(64);
        
        assert_eq!(pos1.len(), 64);
        assert_eq!(pos2.len(), 64);
        assert_ne!(pos1, pos2); // Should be random
        assert!(pos1.chars().all(|c| "abcdef0123456789".contains(c)));
    }

    #[test]
    fn test_key_generation() {
        let key = Wallet::generate_key("0123456789abcdef", "TEST", "position123");
        // JS generateKey returns getHash('HEX', { outputLen: 8192 }) = 2048 hex chars.
        assert_eq!(key.len(), 2048);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_from_response_data() {
        let data = serde_json::json!({
            "balance": 100.0,
            "tokenSlug": "TEST",
            "address": "test-address",
            "bundleHash": "test-bundle",
            "position": "test-position",
            "characters": "BASE64",
            "batchId": "test-batch",
            "tokenUnits": [
                ["unit1", "Unit 1", {"meta": "data"}]
            ]
        });
        
        let wallet = Wallet::from_response_data(data).unwrap();
        
        assert_eq!(wallet.balance, "100");
        assert_eq!(wallet.token, "TEST");
        assert_eq!(wallet.address, Some("test-address".to_string()));
        assert_eq!(wallet.bundle, Some("test-bundle".to_string()));
        assert_eq!(wallet.position, Some("test-position".to_string()));
        assert_eq!(wallet.characters, Some("BASE64".to_string()));
        assert_eq!(wallet.batch_id, Some("test-batch".to_string()));
        assert_eq!(wallet.token_units.len(), 1);
        assert_eq!(wallet.token_units[0].id, "unit1");
    }

    #[test]
    fn test_set_key_from_secret() {
        let mut wallet = Wallet::default();
        wallet.set_key_from_secret("test-secret", "TEST", "test-position").unwrap();
        
        assert_eq!(wallet.token, "TEST");
        assert_eq!(wallet.position, Some("test-position".to_string()));
        assert!(wallet.bundle.is_some());
        assert!(wallet.key.is_some());
        assert!(wallet.address.is_some());
        assert_eq!(wallet.characters, Some("BASE64".to_string()));
    }

    #[test]
    fn test_init_batch_id_from_source() {
        let source_wallet = {
            let mut wallet = Wallet::default();
            wallet.batch_id = Some("source-batch".to_string());
            wallet
        };
        
        let mut target_wallet = Wallet::default();
        target_wallet.init_batch_id_from_source(&source_wallet);
        
        assert_eq!(target_wallet.batch_id, Some("source-batch".to_string()));
    }

    // ============================================================================
    // ML-KEM Quantum Encryption Tests (GAP-07-007)
    // ============================================================================

    #[test]
    fn test_mlkem_deterministic_keygen() {
        // Same secret + token + position → same keypair every time
        let wallet1 = Wallet::create(
            Some("KNISHIO_TEST_SECRET_DO_NOT_USE_IN_PRODUCTION"),
            None, "TEST", Some("fixed_position_for_testing"), None, None,
        ).unwrap();
        let wallet2 = Wallet::create(
            Some("KNISHIO_TEST_SECRET_DO_NOT_USE_IN_PRODUCTION"),
            None, "TEST", Some("fixed_position_for_testing"), None, None,
        ).unwrap();

        assert!(wallet1.pubkey.is_some(), "ML-KEM pubkey should be generated");
        assert!(wallet1.privkey.is_some(), "ML-KEM privkey should be generated");
        assert_eq!(wallet1.pubkey, wallet2.pubkey, "Same seed should produce same pubkey");
        assert_eq!(wallet1.privkey, wallet2.privkey, "Same seed should produce same privkey");
    }

    #[test]
    fn test_mlkem_key_sizes() {
        let wallet1024 = Wallet::create(
            Some("KNISHIO_TEST_SECRET_DO_NOT_USE_IN_PRODUCTION"),
            None, "TEST", None, None, None,
        ).unwrap();

        // ML-KEM-1024 spec: pubkey = 1568 bytes, privkey = 3168 bytes
        let pubkey_bytes = base64::engine::general_purpose::STANDARD
            .decode(wallet1024.pubkey.as_ref().unwrap())
            .unwrap();
        let privkey_bytes = wallet1024.privkey.as_ref().unwrap();

        assert_eq!(pubkey_bytes.len(), 1568, "ML-KEM-1024 pubkey should be 1568 bytes");
        assert_eq!(privkey_bytes.len(), 3168, "ML-KEM-1024 privkey should be 3168 bytes");

        // ML-KEM-768 step-back
        let wallet768 = Wallet::create(
            Some("KNISHIO_TEST_SECRET_DO_NOT_USE_IN_PRODUCTION"),
            None, "TEST", None, None, Some(MlKemParameterSet::MlKem768),
        ).unwrap();

        let pubkey_bytes768 = base64::engine::general_purpose::STANDARD
            .decode(wallet768.pubkey.as_ref().unwrap())
            .unwrap();
        let privkey_bytes768 = wallet768.privkey.as_ref().unwrap();

        assert_eq!(pubkey_bytes768.len(), 1184, "ML-KEM-768 pubkey should be 1184 bytes");
        assert_eq!(privkey_bytes768.len(), 2400, "ML-KEM-768 privkey should be 2400 bytes");
    }

    #[tokio::test]
    async fn test_mlkem_encrypt_decrypt_roundtrip() {
        let sender = Wallet::create(
            Some("sender_secret_for_testing_12345"),
            None, "TEST", None, None, None,
        ).unwrap();
        let recipient = Wallet::create(
            Some("recipient_secret_for_testing_12345"),
            None, "TEST", None, None, None,
        ).unwrap();

        let message = serde_json::json!({"hello": "quantum world", "value": 42});
        let recipient_pubkey = recipient.pubkey.as_ref().unwrap();

        let encrypted = sender.encrypt_message(&message, recipient_pubkey).await.unwrap();

        // Ciphertext should be 1568 bytes (base64 encoded) for ML-KEM-1024 default
        let ciphertext_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encrypted.cipher_text)
            .unwrap();
        assert_eq!(ciphertext_bytes.len(), 1568, "ML-KEM-1024 ciphertext should be 1568 bytes");

        // Decrypt with recipient's private key
        let decrypted = recipient.decrypt_message(&encrypted).await.unwrap();
        assert_eq!(decrypted, message, "Decrypted message should match original");
    }

    // PQ-transport hardening: a stale/non-PQ node advertises a ~48-byte `key`; encrypt_message must
    // return a clean error (not feed libcrux a malformed key). The Rust analogue of the cross-SDK guard.
    #[tokio::test]
    async fn test_mlkem_encrypt_rejects_mismatched_key() {
        let sender = Wallet::create(
            Some("sender_secret_for_testing_12345"),
            None, "TEST", None, None, None,
        ).unwrap();
        // 48 bytes base64 — the exact length a pre-PQ validator advertised in its auth `key`.
        let short_key = base64::engine::general_purpose::STANDARD.encode([0u8; 48]);
        let result = sender.encrypt_message(&serde_json::json!({"q": 1}), &short_key).await;
        assert!(result.is_err(), "encrypt_message must reject non-1568 key on 1024 wallet");

        // 768 key (1184 bytes) rejected by 1024 wallet
        let key_768 = base64::engine::general_purpose::STANDARD.encode([0u8; 1184]);
        let result768 = sender.encrypt_message(&serde_json::json!({"q": 1}), &key_768).await;
        assert!(result768.is_err(), "1024 wallet must reject 768 key");
    }

    #[test]
    fn test_mlkem_different_seeds_different_keys() {
        // Use valid hex secrets (generate_key treats secret as hex BigUint)
        let wallet1 = Wallet::create(
            Some("aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000"),
            None, "TEST", None, None, None,
        ).unwrap();
        let wallet2 = Wallet::create(
            Some("bbbb1111bbbb1111bbbb1111bbbb1111bbbb1111bbbb1111bbbb1111bbbb1111"),
            None, "TEST", None, None, None,
        ).unwrap();

        assert_ne!(wallet1.pubkey, wallet2.pubkey, "Different secrets should produce different ML-KEM pubkeys");
        assert_ne!(wallet1.privkey, wallet2.privkey, "Different secrets should produce different ML-KEM privkeys");
    }

    #[test]
    fn test_mlkem_shadow_wallet_no_keys() {
        // Shadow wallets (no secret) should NOT have ML-KEM keys
        let wallet = Wallet::new(
            None, Some("test-bundle"), Some("TEST"),
            None, None, None, None, None,
        ).unwrap();

        assert!(wallet.pubkey.is_none(), "Shadow wallet should not have ML-KEM pubkey");
        assert!(wallet.privkey.is_none(), "Shadow wallet should not have ML-KEM privkey");
    }

    #[test]
    fn test_token_unit_management() {
        let mut wallet = Wallet::default();
        let mut remainder_wallet = Wallet::default();
        
        // Add some test token units
        wallet.token_units.push(TokenUnit::new(
            "unit1".to_string(),
            "Unit 1".to_string(),
            None,
        ));
        wallet.token_units.push(TokenUnit::new(
            "unit2".to_string(),
            "Unit 2".to_string(),
            None,
        ));
        
        // Split units
        wallet.split_units(&["unit1".to_string()], &mut remainder_wallet, None);
        
        assert_eq!(wallet.token_units.len(), 1);
        assert_eq!(remainder_wallet.token_units.len(), 1);
        assert_eq!(wallet.token_units[0].id, "unit1");
        assert_eq!(remainder_wallet.token_units[0].id, "unit2");
    }

    // ============================================================================
    // Balance Precision Tests (GAP-07-001)
    // ============================================================================

    #[test]
    fn test_balance_precision_large_values() {
        // 9007199254740993 > 2^53 — f64 would round this to 9007199254740992
        let mut wallet = Wallet::default();
        wallet.balance = "9007199254740993".to_string();
        assert_eq!(wallet.balance_as_i128(), 9007199254740993_i128);
        assert_eq!(wallet.balance, "9007199254740993");
    }

    #[test]
    fn test_balance_precision_very_large_values() {
        let mut wallet = Wallet::default();
        wallet.balance = "999999999999999999999".to_string(); // ~10^21, far beyond f64 integer range
        assert_eq!(wallet.balance_as_i128(), 999999999999999999999_i128);
    }

    #[test]
    fn test_balance_helpers() {
        let mut wallet = Wallet::default();
        assert_eq!(wallet.balance, "0");
        assert_eq!(wallet.balance_as_i128(), 0);

        wallet.set_balance_i128(42);
        assert_eq!(wallet.balance, "42");
        assert_eq!(wallet.balance_as_i128(), 42);

        wallet.set_balance_f64(1000.0);
        assert_eq!(wallet.balance, "1000");
    }

    #[test]
    fn test_balance_serde_string_format() {
        // Deserialize from string format (server sends this)
        let json = r#"{"balance":"9007199254740993","token":"TEST","tokenUnits":[],"tradeRates":{},"molecules":{}}"#;
        let wallet: Wallet = serde_json::from_str(json).unwrap();
        assert_eq!(wallet.balance, "9007199254740993");
    }

    #[test]
    fn test_balance_serde_number_format() {
        // Deserialize from number format (backwards compatibility)
        let json = r#"{"balance":1000,"token":"TEST","tokenUnits":[],"tradeRates":{},"molecules":{}}"#;
        let wallet: Wallet = serde_json::from_str(json).unwrap();
        assert_eq!(wallet.balance, "1000");
    }
}