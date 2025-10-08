//! Authentication token management for the KnishIO SDK
//!
//! This module handles authentication tokens for API access.
//! Maintains exact compatibility with JavaScript AuthToken.js implementation.

use serde::{Deserialize, Serialize};
use crate::wallet::Wallet;
use crate::error::Result;

/// Snapshot structure for token restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokenSnapshot {
    pub token: String,
    pub expires_at: Option<i64>,
    pub pubkey: Option<String>,
    pub encrypt: Option<bool>,
    pub wallet: WalletSnapshot,
}

/// Wallet snapshot for auth token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub position: Option<String>,
    pub characters: Option<String>,
}

/// Authentication data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub token: String,
    pub pubkey: Option<String>,
    pub wallet: Option<Wallet>,
}

/// Authentication token structure (matches JS AuthToken exactly)
#[derive(Debug, Clone)]
pub struct AuthToken {
    token: String,
    expires_at: Option<i64>,
    pubkey: Option<String>,
    encrypt: Option<bool>,
    wallet: Option<Wallet>,
}

impl AuthToken {
    /// Create a new authentication token (matches JS constructor)
    ///
    /// # Arguments
    ///
    /// * `token` - Authentication token string
    /// * `expires_at` - Expiration timestamp (optional)
    /// * `encrypt` - Whether encryption is enabled (optional)
    /// * `pubkey` - Public key for encryption (optional)
    pub fn new(
        token: String,
        expires_at: Option<i64>,
        encrypt: Option<bool>,
        pubkey: Option<String>,
    ) -> Self {
        AuthToken {
            token,
            expires_at,
            pubkey,
            encrypt,
            wallet: None,
        }
    }
    
    /// Create AuthToken with data and wallet (matches JS AuthToken.create)
    ///
    /// # Arguments
    ///
    /// * `token` - Authentication token string
    /// * `expires_at` - Expiration timestamp (optional)
    /// * `encrypt` - Whether encryption is enabled (optional)
    /// * `pubkey` - Public key for encryption (optional)
    /// * `wallet` - Associated wallet
    ///
    /// # Returns
    ///
    /// AuthToken instance with wallet set
    pub fn create(
        token: String,
        expires_at: Option<i64>,
        encrypt: Option<bool>,
        pubkey: Option<String>,
        wallet: Wallet,
    ) -> Self {
        let mut auth_token = Self::new(token, expires_at, encrypt, pubkey);
        auth_token.set_wallet(wallet);
        auth_token
    }
    
    /// Restore AuthToken from snapshot and secret (matches JS AuthToken.restore)
    ///
    /// # Arguments
    ///
    /// * `snapshot` - Token snapshot data
    /// * `secret` - Secret for wallet restoration
    ///
    /// # Returns
    ///
    /// Result containing restored AuthToken
    pub fn restore(snapshot: AuthTokenSnapshot, secret: &str) -> Result<Self> {
        let wallet = Wallet::new(
            Some(secret),
            None,
            Some("AUTH"),
            None,
            snapshot.wallet.position.as_deref(),
            None,
            snapshot.wallet.characters.as_deref(),
        )?;
        
        Ok(Self::create(
            snapshot.token,
            snapshot.expires_at,
            snapshot.encrypt,
            snapshot.pubkey,
            wallet,
        ))
    }
    
    /// Set associated wallet (matches JS setWallet)
    ///
    /// # Arguments
    ///
    /// * `wallet` - Wallet to associate with this token
    pub fn set_wallet(&mut self, wallet: Wallet) {
        self.wallet = Some(wallet);
    }
    
    /// Get associated wallet (matches JS getWallet)
    ///
    /// # Returns
    ///
    /// Reference to the associated wallet if set
    pub fn get_wallet(&self) -> Option<&Wallet> {
        self.wallet.as_ref()
    }
    
    /// Get mutable reference to associated wallet
    ///
    /// # Returns
    ///
    /// Mutable reference to the associated wallet if set
    pub fn get_wallet_mut(&mut self) -> Option<&mut Wallet> {
        self.wallet.as_mut()
    }
    
    /// Get snapshot for persistence (matches JS getSnapshot)
    ///
    /// # Returns
    ///
    /// AuthTokenSnapshot for serialization and storage
    pub fn get_snapshot(&self) -> AuthTokenSnapshot {
        let wallet_snapshot = if let Some(ref wallet) = self.wallet {
            WalletSnapshot {
                position: wallet.position.clone(),
                characters: wallet.characters.clone(),
            }
        } else {
            WalletSnapshot {
                position: None,
                characters: None,
            }
        };
        
        AuthTokenSnapshot {
            token: self.token.clone(),
            expires_at: self.expires_at,
            pubkey: self.pubkey.clone(),
            encrypt: self.encrypt,
            wallet: wallet_snapshot,
        }
    }
    
    /// Get the token string (matches JS getToken)
    ///
    /// # Returns
    ///
    /// The authentication token string
    pub fn get_token(&self) -> &str {
        &self.token
    }
    
    /// Get the public key (matches JS getPubkey)
    ///
    /// # Returns
    ///
    /// Optional public key string
    pub fn get_pubkey(&self) -> Option<&str> {
        self.pubkey.as_deref()
    }
    
    /// Get expiration interval in milliseconds (matches JS getExpireInterval)
    ///
    /// # Returns
    ///
    /// Milliseconds until expiration (negative if expired)
    pub fn get_expire_interval(&self) -> i64 {
        if let Some(expires_at) = self.expires_at {
            (expires_at * 1000) - chrono::Utc::now().timestamp_millis()
        } else {
            0
        }
    }
    
    /// Check if the token is expired (matches JS isExpired)
    ///
    /// # Returns
    ///
    /// True if token is expired or has no expiration time
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            self.get_expire_interval() < 0
        } else {
            true // No expiration time means expired
        }
    }
    
    /// Get authentication data for GraphQL client (matches JS getAuthData)
    ///
    /// # Returns
    ///
    /// AuthData structure containing token, pubkey, and wallet
    pub fn get_auth_data(&self) -> AuthData {
        AuthData {
            token: self.token.clone(),
            pubkey: self.pubkey.clone(),
            wallet: self.wallet.clone(),
        }
    }
    
    /// Get the token string (alias for compatibility)
    ///
    /// # Returns
    ///
    /// The authentication token string
    pub fn as_str(&self) -> &str {
        &self.token
    }
    
    /// Get the token field directly (for field access compatibility)
    pub fn token(&self) -> &str {
        &self.token
    }
    
    /// Get the wallet bundle if wallet exists
    pub fn wallet_bundle(&self) -> Option<String> {
        self.wallet.as_ref().and_then(|w| w.bundle.clone())
    }
}

/// Serialize AuthToken for JSON storage
impl Serialize for AuthToken {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let snapshot = self.get_snapshot();
        snapshot.serialize(serializer)
    }
}

/// Deserialize AuthToken from JSON (requires secret for full restoration)
impl<'de> Deserialize<'de> for AuthToken {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let snapshot = AuthTokenSnapshot::deserialize(deserializer)?;
        
        // Create minimal AuthToken without wallet (wallet requires secret)
        Ok(AuthToken {
            token: snapshot.token,
            expires_at: snapshot.expires_at,
            pubkey: snapshot.pubkey,
            encrypt: snapshot.encrypt,
            wallet: None, // Wallet must be restored separately with secret
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_auth_token_creation() {
        let auth_token = AuthToken::new(
            "test-token".to_string(),
            Some(1640995200), // Jan 1, 2022
            Some(true),
            Some("test-pubkey".to_string()),
        );
        
        assert_eq!(auth_token.get_token(), "test-token");
        assert_eq!(auth_token.get_pubkey(), Some("test-pubkey"));
        assert!(auth_token.is_expired()); // Should be expired by now
    }
    
    #[test]
    fn test_auth_token_with_wallet() {
        let wallet = Wallet::create(
            Some("test-secret"),
            None,
            "AUTH",
            None,
            None,
        ).unwrap();
        
        let auth_token = AuthToken::create(
            "test-token".to_string(),
            None,
            Some(false),
            None,
            wallet,
        );
        
        assert!(auth_token.get_wallet().is_some());
        assert_eq!(auth_token.get_wallet().unwrap().token, "AUTH");
    }
    
    #[test]
    fn test_snapshot() {
        let wallet = Wallet::create(
            Some("test-secret"),
            None,
            "AUTH",
            None,
            None,
        ).unwrap();
        
        let auth_token = AuthToken::create(
            "test-token".to_string(),
            Some(1640995200),
            Some(true),
            Some("test-pubkey".to_string()),
            wallet,
        );
        
        let snapshot = auth_token.get_snapshot();
        
        assert_eq!(snapshot.token, "test-token");
        assert_eq!(snapshot.expires_at, Some(1640995200));
        assert_eq!(snapshot.pubkey, Some("test-pubkey".to_string()));
        assert_eq!(snapshot.encrypt, Some(true));
        assert!(snapshot.wallet.position.is_some());
    }
    
    #[test]
    fn test_auth_data() {
        let wallet = Wallet::create(
            Some("test-secret"),
            None,
            "AUTH",
            None,
            None,
        ).unwrap();
        
        let auth_token = AuthToken::create(
            "test-token".to_string(),
            None,
            Some(false),
            Some("test-pubkey".to_string()),
            wallet,
        );
        
        let auth_data = auth_token.get_auth_data();
        
        assert_eq!(auth_data.token, "test-token");
        assert_eq!(auth_data.pubkey, Some("test-pubkey".to_string()));
        assert!(auth_data.wallet.is_some());
    }
    
    #[test]
    fn test_expiration() {
        // Future expiration
        let future_timestamp = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
        let auth_token = AuthToken::new(
            "test-token".to_string(),
            Some(future_timestamp),
            None,
            None,
        );
        
        assert!(!auth_token.is_expired());
        assert!(auth_token.get_expire_interval() > 0);
        
        // Past expiration
        let auth_token_expired = AuthToken::new(
            "test-token".to_string(),
            Some(1640995200), // Jan 1, 2022 (past)
            None,
            None,
        );
        
        assert!(auth_token_expired.is_expired());
        assert!(auth_token_expired.get_expire_interval() < 0);
    }
}