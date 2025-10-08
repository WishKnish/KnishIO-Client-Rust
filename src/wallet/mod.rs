//! Wallet module for the KnishIO SDK
//!
//! This module provides the Wallet struct and associated methods for wallet
//! management, ensuring exact compatibility with the JavaScript implementation.

use crate::crypto::{generate_bundle_hash, generate_key, generate_address};
use crate::types::TokenUnit;
use crate::error::{KnishIOError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};
use aes::cipher::generic_array::GenericArray;
use rand::RngCore;

/// Wallet structure representing cryptographic keys and token management
///
/// The Wallet struct maintains exact compatibility with the JavaScript implementation,
/// including shadow wallet support, ML-KEM quantum encryption, and token unit management.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    /// Token slug this wallet is intended for (e.g., "USER", "TEST")
    pub token: String,
    
    /// Current balance of the wallet
    pub balance: f64,
    
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
    pub fn new(
        secret: Option<&str>,
        bundle: Option<&str>,
        token: Option<&str>,
        address: Option<&str>,
        position: Option<&str>,
        batch_id: Option<&str>,
        characters: Option<&str>,
    ) -> Result<Self> {
        let token = token.unwrap_or("USER").to_string();
        
        let mut wallet = Wallet {
            token: token.clone(),
            balance: 0.0,
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
    pub fn create(
        secret: Option<&str>,
        bundle: Option<&str>,
        token: &str,
        position: Option<&str>,
        characters: Option<&str>,
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
            } else {
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
        // Use pattern matching for cleaner data extraction
        let (balance, token, address, bundle, position, characters, batch_id) = (
            data["balance"].as_f64().unwrap_or(0.0),
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
        )?;

        wallet.balance = balance;

        // Parse token units using iterator chain for better performance
        wallet.token_units = data["tokenUnits"]
            .as_array()
            .map(|units_data| {
                units_data
                    .iter()
                    .filter_map(|unit_data| unit_data.as_array())
                    .filter(|unit_array| unit_array.len() >= 2)
                    .map(|unit_array| {
                        let id = unit_array[0].as_str().unwrap_or("").to_string();
                        let name = unit_array[1].as_str().map(|s| s.to_string()).unwrap_or_default();
                        
                        let meta = (unit_array.len() > 2)
                            .then(|| unit_array[2].as_object())
                            .flatten()
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
                        
                        TokenUnit::new(id, name, meta)
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
        use rand::Rng;
        
        const HEX_CHARSET: &[u8] = b"abcdef0123456789";
        
        let mut rng = rand::thread_rng();
        
        // Use iterator with random sampling for better performance
        (0..salt_length)
            .map(|_| HEX_CHARSET[rng.gen_range(0..HEX_CHARSET.len())] as char)
            .collect()
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
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:016x}", rng.gen::<u64>())
    }

    /// Initialize ML-KEM quantum encryption keys
    ///
    /// Sets up the quantum-resistant encryption key pair using ML-KEM768.
    /// Matches JavaScript implementation using deterministic seed from wallet key.
    fn initialize_mlkem(&mut self) -> Result<()> {
        if let Some(key) = &self.key {
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
            
            // Generate ML-KEM768 key pair using deterministic seed
            use libcrux_ml_kem::mlkem768;
            let keypair = mlkem768::generate_key_pair(seed);
            
            // Serialize keys to match JavaScript base64 format
            self.pubkey = Some(base64::engine::general_purpose::STANDARD.encode(keypair.pk().as_slice()));
            self.privkey = Some(keypair.sk().as_slice().to_vec());
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
            
        // Perform ML-KEM768 encapsulation to get shared secret
        use libcrux_ml_kem::mlkem768;
        use libcrux_ml_kem::{MlKemPublicKey, MlKemCiphertext, MlKemSharedSecret};
        
        // Convert recipient public key bytes to proper libcrux type
        if recipient_pubkey_bytes.len() != 1184 {
            return Err(KnishIOError::DecryptionKey);
        }
        let mut public_key_array = [0u8; 1184];
        public_key_array.copy_from_slice(&recipient_pubkey_bytes);
        let public_key = MlKemPublicKey::from(public_key_array);
        
        // Generate random bytes for encapsulation
        let mut randomness = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut randomness);
        
        let (ciphertext, shared_secret) = mlkem768::encapsulate(&public_key, randomness);
        
        // Encrypt message using AES-GCM with shared secret
        let encrypted_message_bytes = self.encrypt_with_shared_secret(message_bytes, shared_secret.as_slice()).await?;
        
        // Serialize to base64 (matches JavaScript serialization)
        let cipher_text = base64::engine::general_purpose::STANDARD.encode(ciphertext.as_slice());
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
        // Get private key
        let privkey = self.privkey.as_ref()
            .ok_or(KnishIOError::DecryptionKey)?;
            
        // Deserialize ciphertext from base64
        let ciphertext_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encrypted_data.cipher_text)
            .map_err(|_| KnishIOError::DecryptionKey)?;
            
        // Perform ML-KEM768 decapsulation to recover shared secret
        use libcrux_ml_kem::mlkem768;
        use libcrux_ml_kem::{MlKemPrivateKey, MlKemCiphertext};
        
        // Convert secret key to proper libcrux type
        if privkey.len() != 2400 {
            return Err(KnishIOError::DecryptionKey);
        }
        let mut secret_key_array = [0u8; 2400];
        secret_key_array.copy_from_slice(privkey);
        let secret_key = MlKemPrivateKey::from(secret_key_array);
        
        // Convert ciphertext to proper libcrux type
        if ciphertext_bytes.len() != 1088 {
            return Err(KnishIOError::DecryptionKey);
        }
        let mut ciphertext_array = [0u8; 1088];
        ciphertext_array.copy_from_slice(&ciphertext_bytes);
        let ciphertext = MlKemCiphertext::from(ciphertext_array);
        
        let shared_secret = mlkem768::decapsulate(&secret_key, &ciphertext);
        
        // Deserialize encrypted message from base64
        let encrypted_message_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encrypted_data.encrypted_message)
            .map_err(|_| KnishIOError::DecryptionKey)?;
            
        // Decrypt message using AES-GCM with shared secret
        let decrypted_bytes = self.decrypt_with_shared_secret(&encrypted_message_bytes, shared_secret.as_slice()).await?;
        
        // Convert back to JSON
        let decrypted_string = String::from_utf8(decrypted_bytes)
            .map_err(|_| KnishIOError::DecryptionKey)?;
            
        let message = serde_json::from_str::<serde_json::Value>(&decrypted_string)
            .map_err(|_| KnishIOError::DecryptionKey)?;
        
        Ok(message)
    }

    /// Encrypt data with AES-256-GCM using shared secret
    async fn encrypt_with_shared_secret(&self, message: &[u8], shared_secret: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use aes_gcm::aead::Aead;
        
        // Use shared secret as AES key
        let key = GenericArray::from_slice(shared_secret);
        let cipher = Aes256Gcm::new(key);
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt message
        let mut encrypted = cipher.encrypt(nonce, message)
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
        let key = GenericArray::from_slice(shared_secret);
        let cipher = Aes256Gcm::new(key);
        
        // Extract nonce and encrypted message
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let encrypted_message = &encrypted_data[12..];
        
        // Decrypt message
        let decrypted = cipher.decrypt(nonce, encrypted_message)
            .map_err(|_| KnishIOError::DecryptionKey)?;
        
        Ok(decrypted)
    }
}

/// Encrypted message structure for quantum encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessage {
    pub cipher_text: String,
    pub encrypted_message: String,
}

impl Default for Wallet {
    fn default() -> Self {
        Wallet {
            token: "USER".to_string(),
            balance: 0.0,
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
        assert_eq!(key.len(), 4096);
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
        
        assert_eq!(wallet.balance, 100.0);
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
}