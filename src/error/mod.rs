//! Error types for the KnishIO SDK
//!
//! This module provides a comprehensive error hierarchy that maps exactly to
//! the JavaScript SDK's exception classes. Each error type corresponds to a
//! specific exception in the JS implementation.

use thiserror::Error;

/// Main error type for the KnishIO SDK
///
/// This enum contains all possible errors that can occur during SDK operations,
/// maintaining exact compatibility with the JavaScript exception hierarchy.
#[derive(Error, Debug, Clone)]
pub enum KnishIOError {
    // Atom-related errors
    
    /// Atom index is out of bounds or invalid
    #[error("Atom index out of bounds")]
    AtomIndex,
    
    /// Required atoms are missing from the molecule
    #[error("Atoms missing from molecule")]
    AtomsMissing,
    
    // Authorization errors
    
    /// Authorization request was rejected
    #[error("Authorization rejected")]
    AuthorizationRejected,
    
    // Balance errors
    
    /// Insufficient balance for the requested operation
    #[error("Insufficient balance")]
    BalanceInsufficient,
    
    // Batch errors
    
    /// Invalid or malformed batch ID
    #[error("Invalid batch ID")]
    BatchId,
    
    // Code errors
    
    /// Invalid code provided
    #[error("Invalid code: {0}")]
    Code(String),
    
    // Cryptographic errors
    
    /// Decryption key error
    #[error("Decryption key error")]
    DecryptionKey,
    
    /// Encryption error
    #[error("Encryption error")]
    EncryptionError,
    
    /// Invalid key format or size
    #[error("Invalid key")]
    InvalidKey,
    
    // Response errors
    
    /// Invalid response received from server
    #[error("Invalid response from server")]
    InvalidResponse,
    
    // Metadata errors
    
    /// Required metadata is missing
    #[error("Required metadata missing")]
    MetaMissing,
    
    // Molecular errors
    
    /// Molecular hash does not match expected value
    #[error("Molecular hash mismatch")]
    MolecularHashMismatch,
    
    /// Molecular hash is missing when required
    #[error("Molecular hash missing")]
    MolecularHashMissing,
    
    // Amount errors
    
    /// Amount cannot be negative
    #[error("Amount cannot be negative")]
    NegativeAmount,
    
    // Policy errors
    
    /// Policy validation failed
    #[error("Invalid policy")]
    PolicyInvalid,
    
    // Signature errors
    
    /// Signature is malformed or corrupted
    #[error("Signature malformed")]
    SignatureMalformed,
    
    /// Signature does not match expected value
    #[error("Signature mismatch")]
    SignatureMismatch,
    
    // Token unit errors
    
    /// Invalid amount for stackable token unit
    #[error("Invalid stackable unit amount")]
    StackableUnitAmount,
    
    /// Invalid decimal places for stackable token unit
    #[error("Invalid stackable unit decimals")]
    StackableUnitDecimals,
    
    // Transfer errors
    
    /// Transfer balance error
    #[error("Transfer balance error")]
    TransferBalance,
    
    /// Transfer is malformed or invalid
    #[error("Transfer malformed")]
    TransferMalformed,
    
    /// Transfer values do not match
    #[error("Transfer mismatched")]
    TransferMismatched,
    
    /// Transfer remainder calculation error
    #[error("Transfer remainder error")]
    TransferRemainder,
    
    /// Cannot transfer tokens to self
    #[error("Cannot transfer to self")]
    TransferToSelf,
    
    /// Transfer is unbalanced (inputs != outputs)
    #[error("Transfer unbalanced")]
    TransferUnbalanced,
    
    // Authentication errors
    
    /// User is not authenticated
    #[error("Unauthenticated")]
    Unauthenticated,
    
    // Wallet errors
    
    /// Invalid wallet credentials
    #[error("Invalid wallet credentials")]
    WalletCredential,
    
    /// Shadow wallet operation error
    #[error("Shadow wallet error")]
    WalletShadow,

    /// Wallet not found error
    #[error("Wallet not found")]
    WalletNotFound,    

    // Missing resource errors
    
    /// Missing secret for wallet operation
    #[error("Missing secret")]
    MissingSecret,
    
    /// Missing bundle hash
    #[error("Missing bundle")]
    MissingBundle,
    
    /// No client connection available
    #[error("No client")]
    NoClient,
    
    /// Authentication failed
    #[error("Authentication failed")]
    AuthenticationFailed,    // Token type errors
    
    /// Wrong token type for requested operation
    #[error("Wrong token type")]
    WrongTokenType,
    
    // Network and external errors
    
    /// Network communication error
    #[error("Network error: {0}")]
    Network(String),
    
    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Generic I/O error
    #[error("I/O error: {0}")]
    Io(String),
    
    /// UTF-8 encoding error
    #[error("UTF-8 error: {0}")]
    Utf8(String),
    
    /// WebSocket communication error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Configuration or builder validation error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Custom error with message
    #[error("{0}")]
    Custom(String),
}

/// Type alias for Results using KnishIOError
pub type Result<T> = std::result::Result<T, KnishIOError>;

impl KnishIOError {
    /// Create a custom error with a message
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        KnishIOError::Custom(msg.into())
    }
    
    /// Create a network error from a reqwest error
    pub fn from_network_error(error: reqwest::Error) -> Self {
        KnishIOError::Network(error.to_string())
    }
    
    /// Create a serialization error from a serde_json error
    pub fn from_serialization_error(error: serde_json::Error) -> Self {
        KnishIOError::Serialization(error.to_string())
    }
    
    /// Create an I/O error from a std::io error
    pub fn from_io_error(error: std::io::Error) -> Self {
        KnishIOError::Io(error.to_string())
    }
    
    /// Check if this error is a network-related error
    pub fn is_network_error(&self) -> bool {
        matches!(self, KnishIOError::Network(_) | KnishIOError::WebSocketError(_))
    }
    
    /// Check if this error is a cryptographic error
    pub fn is_crypto_error(&self) -> bool {
        matches!(
            self,
            KnishIOError::DecryptionKey
                | KnishIOError::EncryptionError
                | KnishIOError::InvalidKey
                | KnishIOError::SignatureMalformed
                | KnishIOError::SignatureMismatch
                | KnishIOError::MolecularHashMismatch
                | KnishIOError::MolecularHashMissing
        )
    }
    
    /// Check if this error is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            KnishIOError::AtomIndex
                | KnishIOError::AtomsMissing
                | KnishIOError::BatchId
                | KnishIOError::Code(_)
                | KnishIOError::InvalidResponse
                | KnishIOError::MetaMissing
                | KnishIOError::NegativeAmount
                | KnishIOError::PolicyInvalid
                | KnishIOError::StackableUnitAmount
                | KnishIOError::StackableUnitDecimals
                | KnishIOError::TransferMalformed
                | KnishIOError::TransferMismatched
                | KnishIOError::WrongTokenType
        )
    }
    
    /// Check if this error is an authentication error
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            KnishIOError::AuthorizationRejected
                | KnishIOError::Unauthenticated
                | KnishIOError::WalletCredential
        )
    }
    
    /// Check if this error is a balance/transfer error
    pub fn is_balance_error(&self) -> bool {
        matches!(
            self,
            KnishIOError::BalanceInsufficient
                | KnishIOError::TransferBalance
                | KnishIOError::TransferRemainder
                | KnishIOError::TransferToSelf
                | KnishIOError::TransferUnbalanced
        )
    }
}

// Implement From traits for easier error conversion
impl From<reqwest::Error> for KnishIOError {
    fn from(error: reqwest::Error) -> Self {
        KnishIOError::Network(error.to_string())
    }
}

impl From<serde_json::Error> for KnishIOError {
    fn from(error: serde_json::Error) -> Self {
        KnishIOError::Serialization(error.to_string())
    }
}

impl From<std::io::Error> for KnishIOError {
    fn from(error: std::io::Error) -> Self {
        KnishIOError::Io(error.to_string())
    }
}

impl From<std::string::FromUtf8Error> for KnishIOError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        KnishIOError::Utf8(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = KnishIOError::BalanceInsufficient;
        assert_eq!(err.to_string(), "Insufficient balance");
        
        let err = KnishIOError::Code("TEST123".to_string());
        assert_eq!(err.to_string(), "Invalid code: TEST123");
        
        let err = KnishIOError::custom("Custom error message");
        assert_eq!(err.to_string(), "Custom error message");
    }
    
    #[test]
    fn test_error_categories() {
        // Test crypto errors
        assert!(KnishIOError::SignatureMismatch.is_crypto_error());
        assert!(KnishIOError::MolecularHashMismatch.is_crypto_error());
        
        // Test validation errors
        assert!(KnishIOError::AtomsMissing.is_validation_error());
        assert!(KnishIOError::PolicyInvalid.is_validation_error());
        
        // Test auth errors
        assert!(KnishIOError::Unauthenticated.is_auth_error());
        assert!(KnishIOError::AuthorizationRejected.is_auth_error());
        
        // Test balance errors
        assert!(KnishIOError::BalanceInsufficient.is_balance_error());
        assert!(KnishIOError::TransferUnbalanced.is_balance_error());
    }
}