//! Common type definitions for the KnishIO SDK
//!
//! This module contains shared types, enums, and structures used throughout
//! the SDK, ensuring consistent data representation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Isotope types for atomic operations
///
/// Each isotope represents a different type of operation that can be
/// performed within an atom. This enum maps exactly to the isotope
/// classifications in the JavaScript SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Isotope {
    /// Value transfer operations
    #[default]
    V,
    /// Creation operations (tokens, wallets)
    C,
    /// Metadata operations
    M,
    /// Identity/ContinuID operations
    I,
    /// Token request operations
    T,
    /// Authorization operations
    U,
    /// Rule/Policy operations
    R,
    /// Buffer operations
    B,
    /// Fusion operations
    F,
    /// Peering operations
    P,
    /// Append request operations
    A,
}

impl Isotope {
    /// Convert isotope to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Isotope::V => "V",
            Isotope::C => "C",
            Isotope::M => "M",
            Isotope::I => "I",
            Isotope::T => "T",
            Isotope::U => "U",
            Isotope::R => "R",
            Isotope::B => "B",
            Isotope::F => "F",
            Isotope::P => "P",
            Isotope::A => "A",
        }
    }

    /// Parse isotope from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "V" => Some(Isotope::V),
            "C" => Some(Isotope::C),
            "M" => Some(Isotope::M),
            "I" => Some(Isotope::I),
            "T" => Some(Isotope::T),
            "U" => Some(Isotope::U),
            "R" => Some(Isotope::R),
            "B" => Some(Isotope::B),
            "F" => Some(Isotope::F),
            "P" => Some(Isotope::P),
            "A" => Some(Isotope::A),
            _ => None,
        }
    }
}

/// Metadata item structure
///
/// Represents a key-value pair for metadata storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetaItem {
    pub key: String,
    pub value: String,
}

impl MetaItem {
    /// Create a new metadata item
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        MetaItem {
            key: key.into(),
            value: value.into(),
        }
    }
}

// Re-export TokenUnit from the dedicated token_unit module
pub use crate::token_unit::TokenUnit;

/// Balance response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub address: String,
    pub position: String,
    pub amount: f64,
    pub characters: Option<String>,
}

/// ContinuID structure for identity continuity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuId {
    pub bundle_hash: String,
    pub position: String,
    pub batch_id: Option<String>,
}

/// Molecule status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoleculeStatus {
    Pending,
    Accepted,
    Rejected,
}

/// GraphQL variable types
pub type Variables = HashMap<String, serde_json::Value>;

/// Generic GraphQL response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// GraphQL error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

/// Wallet bundle information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBundle {
    pub bundle_hash: String,
    pub created_at: i64,
    pub wallets: Vec<WalletInfo>,
}

/// Basic wallet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub address: String,
    pub token: String,
    pub position: String,
    pub amount: f64,
}

/// Policy structure for access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub read: Option<String>,
    pub write: Option<String>,
    pub execute: Option<String>,
}

/// Rule structure for business logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub condition: String,
    pub action: String,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub bundle_hash: String,
    pub ip_address: Option<String>,
    pub browser: Option<String>,
    pub os_cpu: Option<String>,
    pub resolution: Option<String>,
    pub time_zone: Option<String>,
    pub created_at: i64,
}

/// Batch information for grouped transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchInfo {
    pub batch_id: String,
    pub status: String,
    pub created_at: i64,
    pub molecules: Vec<String>,
}

/// Trade rate for buffer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeRate {
    pub source_token: String,
    pub target_token: String,
    pub rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isotope_conversion() {
        assert_eq!(Isotope::V.as_str(), "V");
        assert_eq!(Isotope::from_str("V"), Some(Isotope::V));
        assert_eq!(Isotope::from_str("X"), None);
    }
    
    #[test]
    fn test_meta_item() {
        let meta = MetaItem::new("key", "value");
        assert_eq!(meta.key, "key");
        assert_eq!(meta.value, "value");
    }
    
    #[test]
    fn test_isotope_serialization() {
        let isotope = Isotope::V;
        let json = serde_json::to_string(&isotope).unwrap();
        assert_eq!(json, "\"V\"");
        
        let deserialized: Isotope = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, isotope);
    }
}
/// Options for Atom JSON serialization (2025 Rust patterns)
#[derive(Debug, Clone)]
pub struct AtomJsonOptions {
    /// Include OTS signature fragments (default: true)
    pub include_ots_fragments: bool,
    /// Validate required fields before serialization (default: false) 
    pub validate_fields: bool,
}

impl Default for AtomJsonOptions {
    fn default() -> Self {
        Self {
            include_ots_fragments: true,
            validate_fields: false,
        }
    }
}

/// Options for Atom JSON deserialization (2025 Rust patterns)
#[derive(Debug, Clone)]
pub struct AtomFromJsonOptions {
    /// Validate required fields during deserialization (default: true)
    pub validate_structure: bool,
    /// Strict validation mode (default: false)
    pub strict_mode: bool,
}

impl Default for AtomFromJsonOptions {
    fn default() -> Self {
        Self {
            validate_structure: true,
            strict_mode: false,
        }
    }
}

/// Options for Molecule JSON serialization (2025 Rust patterns)
#[derive(Debug, Clone)]
pub struct MoleculeJsonOptions {
    /// Include sourceWallet/remainderWallet for validation (default: true)
    pub include_validation_context: bool,
    /// Include OTS signature fragments (default: true)
    pub include_ots_fragments: bool,
    /// Extra security checks (default: false)
    pub secure_mode: bool,
}

impl Default for MoleculeJsonOptions {
    fn default() -> Self {
        Self {
            include_validation_context: true,
            include_ots_fragments: true,
            secure_mode: false,
        }
    }
}

/// Options for Molecule JSON deserialization (2025 Rust patterns)
#[derive(Debug, Clone)]
pub struct MoleculeFromJsonOptions {
    /// Reconstruct sourceWallet/remainderWallet (default: true)
    pub include_validation_context: bool,
    /// Validate required fields (default: true)
    pub validate_structure: bool,
    /// Strict validation mode (default: false)
    pub strict_mode: bool,
}

impl Default for MoleculeFromJsonOptions {
    fn default() -> Self {
        Self {
            include_validation_context: true,
            validate_structure: true,
            strict_mode: false,
        }
    }
}
