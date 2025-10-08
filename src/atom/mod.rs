//! Atom module for KnishIO SDK
//!
//! This module provides the Atom struct and related functionality for creating
//! microtransactions within Molecules. The implementation maintains 100%
//! compatibility with the JavaScript SDK.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::crypto::{shake256, shake256_incremental, hex_to_base17};
use crate::error::KnishIOError;
use crate::types::{Isotope, MetaItem};

/// Represents a single atomic operation within a molecular transaction
///
/// Atoms are the fundamental units of the KnishIO distributed ledger,
/// representing individual operations that can be grouped into molecules.
/// This implementation ensures exact compatibility with the JavaScript SDK.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Atom {
    /// Position of the wallet (required)
    pub position: String,
    
    /// Wallet address (required)
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,
    
    /// Isotope type indicating the operation type (required)
    pub isotope: Isotope,
    
    /// Token slug (required)
    pub token: String,
    
    /// Value for the operation (optional, stored as string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    
    /// Batch ID for grouping related operations (optional)
    #[serde(rename = "batchId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    
    /// Metadata type (optional)
    #[serde(rename = "metaType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_type: Option<String>,
    
    /// Metadata ID (optional)
    #[serde(rename = "metaId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_id: Option<String>,
    
    /// Metadata key-value pairs (always present but may be empty)
    pub meta: Vec<MetaItem>,
    
    /// OTS fragment for quantum-resistant signatures (excluded from hashing)
    #[serde(rename = "otsFragment")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ots_fragment: Option<String>,
    
    /// Index for sorting atoms within a molecule (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    
    /// Creation timestamp (automatically set)
    #[serde(rename = "createdAt")]
    pub created_at: String,
    
    /// Version identifier (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// Note: Custom OTS fragment serialization is handled by skip_serializing_if

impl Atom {
    /// Create a new Atom with required fields
    ///
    /// # Arguments
    ///
    /// * `position` - Wallet position string
    /// * `wallet_address` - Wallet address string
    /// * `isotope` - Operation isotope type
    /// * `token` - Token slug
    ///
    /// # Returns
    ///
    /// A new Atom instance with default values for optional fields
    pub fn new(
        position: impl Into<String>,
        wallet_address: impl Into<String>,
        isotope: Isotope,
        token: impl Into<String>,
    ) -> Self {
        let timestamp = Self::generate_timestamp();
        
        Atom {
            position: position.into(),
            wallet_address: wallet_address.into(),
            isotope,
            token: token.into(),
            value: None,
            batch_id: None,
            meta_type: None,
            meta_id: None,
            meta: Vec::new(),
            ots_fragment: None,
            index: None,
            created_at: timestamp,
            version: None,
        }
    }
    
    /// Generate timestamp for atom creation
    /// Use environment variable for deterministic testing
    fn generate_timestamp() -> String {
        if let Ok(fixed_time) = std::env::var("KNISHIO_FIXED_TIMESTAMP") {
            fixed_time
        } else {
            // JavaScript: String(+new Date()) - milliseconds since epoch
            chrono::Utc::now().timestamp_millis().to_string()
        }
    }
    
    /// Create an Atom using the builder pattern (matches JS Atom.create)
    ///
    /// # Arguments
    ///
    /// * `isotope` - Operation isotope type
    /// * `wallet` - Optional wallet reference (contains position, address, token)
    /// * `value` - Optional value for the operation
    /// * `meta_type` - Optional metadata type
    /// * `meta_id` - Optional metadata ID
    /// * `meta` - Optional metadata items
    /// * `batch_id` - Optional batch ID
    ///
    /// # Returns
    ///
    /// A new Atom instance configured with the provided parameters
    pub fn create(params: AtomCreateParams) -> Self {
        let timestamp = Self::generate_timestamp();
        
        let mut atom = Atom {
            position: params.position.unwrap_or_default(),
            wallet_address: params.wallet_address.unwrap_or_default(),
            isotope: params.isotope,
            token: params.token.unwrap_or_default(),
            value: params.value.map(|v| v.to_string()),
            batch_id: params.batch_id,
            meta_type: params.meta_type,
            meta_id: params.meta_id,
            meta: params.meta.unwrap_or_default(),
            ots_fragment: None,
            index: params.index,
            created_at: timestamp,
            version: params.version,
        };
        
        // If wallet info provided, use it
        if let Some(wallet_info) = params.wallet_info {
            atom.position = wallet_info.position;
            atom.wallet_address = wallet_info.address;
            atom.token = wallet_info.token;
            if atom.batch_id.is_none() {
                atom.batch_id = wallet_info.batch_id;
            }
        }
        
        atom
    }
    
    /// Convert JSON string to Atom object (matches JS Atom.jsonToObject)
    ///
    /// # Arguments
    ///
    /// * `json` - JSON string representation of an Atom
    ///
    /// # Returns
    ///
    /// Result containing the deserialized Atom or an error
    pub fn json_to_object(json: &str) -> std::result::Result<Atom, KnishIOError> {
        let atom: Atom = serde_json::from_str(json)?;
        Ok(atom)
    }
    
    /// Get the list of properties that are included in hashing
    ///
    /// This matches the JavaScript implementation exactly, excluding `otsFragment`
    /// and `index` from the hashable properties.
    ///
    /// # Returns
    ///
    /// Vector of property names that should be included in hash calculations
    pub fn get_hashable_props() -> Vec<&'static str> {
        vec![
            "position",
            "walletAddress",
            "isotope",
            "token",
            "value",
            "batchId",
            "metaType",
            "metaId",
            "meta",
            "createdAt",
        ]
    }
    
    /// Get the list of properties that are unclaimed (excluded from serialization in some contexts)
    ///
    /// # Returns
    ///
    /// Vector of property names that are considered unclaimed
    pub fn get_unclaimed_props() -> Vec<&'static str> {
        vec!["otsFragment"]
    }
    
    /// Hash a collection of atoms to produce a molecular hash
    ///
    /// This function produces the same output as the JavaScript implementation
    /// and is used to generate the molecularHash field for Molecules.
    ///
    /// # Arguments
    ///
    /// * `atoms` - Vector of atoms to hash
    /// * `output` - Output format ("hex", "array", or "base17")
    ///
    /// # Returns
    ///
    /// Result containing the hash in the requested format
    pub fn hash_atoms(atoms: &[Atom], output: &str) -> std::result::Result<String, KnishIOError> {
        if atoms.is_empty() {
            return Err(KnishIOError::AtomsMissing);
        }
        
        // Sort atoms using JavaScript-compatible sorting
        let sorted_atoms = Self::sort_atoms(atoms);
        
        // Check if all atoms have versions and use versioned hashing if so
        let all_have_versions = sorted_atoms.iter().all(|atom| atom.version.is_some());
        
        let hex_hash = if all_have_versions {
            // Use versioned view (placeholder for now - YAGNI)
            // TODO: Implement when needed for versioned molecules
            let atom_views: Vec<serde_json::Value> = sorted_atoms.iter()
                .map(|atom| serde_json::to_value(atom))
                .collect::<Result<Vec<_>, _>>()?;
            shake256(&serde_json::to_string(&atom_views)?, 256)
        } else {
            // JavaScript legacy hashing: exact match pattern
            let num_atoms = atoms.len().to_string();  // Use original length, not sorted
            let mut hash_values = Vec::new();
            
            for atom in &sorted_atoms {
                // Add number of atoms for EACH atom (matches JS "Add number of atoms (???)")
                hash_values.push(num_atoms.clone());
                
                // Add atom's hashable values
                hash_values.extend(atom.get_hashable_values());
            }
            
            // Use incremental hashing to match JavaScript SDK exactly
            shake256_incremental(&hash_values, 256)
        };
        
        // Return in requested format
        match output {
            "hex" => Ok(hex_hash),
            "array" => {
                // Convert hex to byte array representation
                let bytes: std::result::Result<Vec<u8>, _> = (0..hex_hash.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex_hash[i..i + 2], 16))
                    .collect();
                match bytes {
                    Ok(b) => Ok(format!("{:?}", b)),
                    Err(_) => Err(KnishIOError::custom("Failed to convert hex to bytes")),
                }
            }
            _ => {
                // Default: base17 representation - matches JS charsetBaseConvert exactly
                Ok(hex_to_base17(&hex_hash))
            }
        }
    }
    
    /// Sort atoms by their index field - matches JavaScript implementation exactly
    ///
    /// # Arguments
    ///
    /// * `atoms` - Vector of atoms to sort
    ///
    /// # Returns
    ///
    /// Vector of atoms sorted by index
    pub fn sort_atoms(atoms: &[Atom]) -> Vec<Atom> {
        let mut sorted = atoms.to_vec();
        // JavaScript: first.index < second.index ? -1 : 1
        sorted.sort_by(|first, second| {
            let first_index = first.index.unwrap_or(0);
            let second_index = second.index.unwrap_or(0);
            if first_index < second_index {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });
        sorted
    }
    
    /// Get the hashable values for this atom - matches JavaScript implementation exactly
    ///
    /// This method extracts values from the atom's properties that should be
    /// included in hash calculations, following the JavaScript implementation exactly.
    ///
    /// # Returns
    ///
    /// Vector of strings representing the hashable values
    pub fn get_hashable_values(&self) -> Vec<String> {
        let mut hashable_values = Vec::new();
        
        // Process properties in the exact order as JavaScript getHashableProps()
        for property in Self::get_hashable_props() {
            let value = self.get_property_value(property);
            
            // JavaScript: All nullable values are not hashed (only custom keys)
            // Skip null values except for position and walletAddress
            if value.is_none() && !["position", "walletAddress"].contains(&property) {
                continue;
            }
            
            // JavaScript: Hashing individual meta keys and values
            if property == "meta" {
                for meta_item in &self.meta {
                    // JavaScript: if (typeof meta.value !== 'undefined' && meta.value !== null)
                    if !meta_item.value.is_empty() {
                        hashable_values.push(meta_item.key.clone());
                        hashable_values.push(meta_item.value.clone());
                    }
                }
            } else {
                // JavaScript: value === null ? '' : String(value)
                hashable_values.push(value.unwrap_or_else(|| String::new()));
            }
        }
        
        hashable_values
    }
    
    /// Get aggregated metadata from stored normalized metadata
    ///
    /// # Returns
    ///
    /// HashMap containing aggregated metadata
    pub fn aggregated_meta(&self) -> HashMap<String, String> {
        let mut aggregated = HashMap::new();
        
        for meta_item in &self.meta {
            aggregated.insert(meta_item.key.clone(), meta_item.value.clone());
        }
        
        aggregated
    }
    
    /// Enhanced JSON serialization for cross-SDK compatibility (Rust 2025 best practices)
    ///
    /// Provides clean serialization of atomic operations with optional OTS fragments.
    /// Follows JavaScript canonical format while using Rust language patterns.
    ///
    /// # Arguments
    ///
    /// * `options` - Serialization options
    ///
    /// # Returns
    ///
    /// Result containing JSON-serializable Value
    pub fn to_json(&self, options: crate::types::AtomJsonOptions) -> Result<serde_json::Value, KnishIOError> {
        // Validate required fields if requested
        if options.validate_fields {
            let required_fields = vec!["position", "walletAddress", "isotope", "token"];
            for field in required_fields {
                match field {
                    "position" if self.position.is_empty() => {
                        return Err(KnishIOError::custom("Required field 'position' is missing or empty"));
                    }
                    "walletAddress" if self.wallet_address.is_empty() => {
                        return Err(KnishIOError::custom("Required field 'walletAddress' is missing or empty"));
                    }
                    "token" if self.token.is_empty() => {
                        return Err(KnishIOError::custom("Required field 'token' is missing or empty"));
                    }
                    _ => {}
                }
            }
        }

        // Build core atom properties (always included) - matches JavaScript exactly
        let mut serialized = serde_json::json!({
            "position": self.position,
            "walletAddress": self.wallet_address,
            "isotope": self.isotope.as_str(),
            "token": self.token,
            "value": self.value,
            "batchId": self.batch_id,
            "metaType": self.meta_type,
            "metaId": self.meta_id,
            "meta": self.meta,
            "index": self.index,
            "createdAt": self.created_at,
            "version": self.version
        });

        // Optional OTS fragments (can be large, so optional)
        if options.include_ots_fragments {
            if let Some(ref ots_fragment) = self.ots_fragment {
                serialized["otsFragment"] = serde_json::Value::String(ots_fragment.clone());
            }
        }

        Ok(serialized)
    }

    /// Enhanced JSON deserialization for cross-SDK compatibility (Rust 2025 best practices)
    /// 
    /// Handles cross-SDK atom deserialization with robust error handling.
    /// Essential for reconstructing atoms from other SDK implementations.
    ///
    /// # Arguments
    ///
    /// * `json` - JSON Value to deserialize
    /// * `options` - Deserialization options
    ///
    /// # Returns
    ///
    /// Result containing reconstructed Atom instance
    pub fn from_json(json: &serde_json::Value, options: crate::types::AtomFromJsonOptions) -> Result<Self, KnishIOError> {
        // Validate required fields in strict mode
        if options.strict_mode || options.validate_structure {
            let required_fields = vec!["position", "walletAddress", "isotope", "token"];
            for field in required_fields {
                if json.get(field).and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                    return Err(KnishIOError::custom(&format!(
                        "Required field '{}' is missing or empty", field
                    )));
                }
            }
        }

        // Extract required fields
        let position = json.get("position")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let wallet_address = json.get("walletAddress")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let isotope_str = json.get("isotope")
            .and_then(|v| v.as_str())
            .unwrap_or("V");
        
        let isotope = Isotope::from_str(isotope_str)
            .ok_or_else(|| KnishIOError::custom(&format!("Invalid isotope: {}", isotope_str)))?;

        let token = json.get("token")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // Create atom instance with required fields
        let mut atom = Atom::new(position, wallet_address, isotope, token);

        // Set optional properties
        if let Some(value) = json.get("value").and_then(|v| v.as_str()) {
            atom.value = Some(value.to_string());
        }

        if let Some(batch_id) = json.get("batchId").and_then(|v| v.as_str()) {
            atom.batch_id = Some(batch_id.to_string());
        }

        if let Some(meta_type) = json.get("metaType").and_then(|v| v.as_str()) {
            atom.meta_type = Some(meta_type.to_string());
        }

        if let Some(meta_id) = json.get("metaId").and_then(|v| v.as_str()) {
            atom.meta_id = Some(meta_id.to_string());
        }

        if let Some(index) = json.get("index").and_then(|v| v.as_u64()) {
            atom.index = Some(index as u32);
        }

        if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
            atom.version = Some(version.to_string());
        }

        // Handle meta array
        if let Some(meta_array) = json.get("meta").and_then(|v| v.as_array()) {
            atom.meta = meta_array.iter()
                .filter_map(|meta_item| {
                    if let (Some(key), Some(value)) = (
                        meta_item.get("key").and_then(|k| k.as_str()),
                        meta_item.get("value").and_then(|v| v.as_str())
                    ) {
                        Some(MetaItem {
                            key: key.to_string(),
                            value: value.to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
        }

        // Set additional properties that may not be in constructor
        if let Some(ots_fragment) = json.get("otsFragment").and_then(|v| v.as_str()) {
            atom.ots_fragment = Some(ots_fragment.to_string());
        }

        if let Some(created_at) = json.get("createdAt").and_then(|v| v.as_str()) {
            atom.created_at = created_at.to_string();
        }

        Ok(atom)
    }
    
    /// Configure optional fields for the atom (builder pattern)
    ///
    /// This method allows setting optional fields after creating an atom with
    /// the basic required fields, following a fluent builder pattern.
    ///
    /// # Arguments
    ///
    /// * `value` - Optional value for the operation
    /// * `batch_id` - Optional batch ID for grouping related operations
    /// * `meta_type` - Optional metadata type
    /// * `meta_id` - Optional metadata ID
    /// * `meta` - Optional metadata key-value pairs
    ///
    /// # Returns
    ///
    /// The atom with optional fields configured
    pub fn with_optional_fields(
        mut self,
        value: Option<f64>,
        batch_id: Option<&str>,
        meta_type: Option<&str>,
        meta_id: Option<&str>,
        meta: Option<Vec<MetaItem>>,
    ) -> Self {
        self.value = value.map(|v| v.to_string());
        self.batch_id = batch_id.map(|b| b.to_string());
        self.meta_type = meta_type.map(|mt| mt.to_string());
        self.meta_id = meta_id.map(|mi| mi.to_string());
        self.meta = meta.unwrap_or_default();
        self
    }

    /// Helper method to get property value as Option<String>
    fn get_property_value(&self, property: &str) -> Option<String> {
        match property {
            "position" => Some(self.position.clone()),
            "walletAddress" => Some(self.wallet_address.clone()),
            "isotope" => Some(self.isotope.as_str().to_string()),
            "token" => Some(self.token.clone()),
            "value" => self.value.clone(),
            "batchId" => self.batch_id.clone(),
            "metaType" => self.meta_type.clone(),
            "metaId" => self.meta_id.clone(),
            "createdAt" => Some(self.created_at.clone()),
            "meta" => Some("meta".to_string()), // Special marker for meta handling
            _ => None,
        }
    }
}

/// Parameters for creating an Atom using the builder pattern
#[derive(Debug)]
pub struct AtomCreateParams {
    pub isotope: Isotope,
    pub position: Option<String>,
    pub wallet_address: Option<String>,
    pub token: Option<String>,
    pub value: Option<f64>,
    pub meta_type: Option<String>,
    pub meta_id: Option<String>,
    pub meta: Option<Vec<MetaItem>>,
    pub batch_id: Option<String>,
    pub index: Option<u32>,
    pub version: Option<String>,
    pub wallet_info: Option<WalletInfo>,
}

/// Wallet information for atom creation
#[derive(Debug, Clone)]
pub struct WalletInfo {
    pub position: String,
    pub address: String,
    pub token: String,
    pub batch_id: Option<String>,
}

impl Default for AtomCreateParams {
    fn default() -> Self {
        AtomCreateParams {
            isotope: Isotope::V,
            position: None,
            wallet_address: None,
            token: None,
            value: None,
            meta_type: None,
            meta_id: None,
            meta: None,
            batch_id: None,
            index: None,
            version: None,
            wallet_info: None,
        }
    }
}


impl Default for Atom {
    fn default() -> Self {
        Atom::new("", "", Isotope::V, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MetaItem;

    #[test]
    fn test_atom_creation() {
        let atom = Atom::new("pos123", "addr456", Isotope::V, "TEST");
        
        assert_eq!(atom.position, "pos123");
        assert_eq!(atom.wallet_address, "addr456");
        assert_eq!(atom.isotope, Isotope::V);
        assert_eq!(atom.token, "TEST");
        assert!(atom.value.is_none());
        assert!(atom.batch_id.is_none());
        assert!(!atom.created_at.is_empty());
    }
    
    #[test]
    fn test_atom_create_with_params() {
        let meta = vec![MetaItem::new("key1", "value1")];
        let wallet_info = WalletInfo {
            position: "wallet_pos".to_string(),
            address: "wallet_addr".to_string(),
            token: "WALLET_TOKEN".to_string(),
            batch_id: Some("batch123".to_string()),
        };
        
        let params = AtomCreateParams {
            isotope: Isotope::M,
            value: Some(100.0),
            meta_type: Some("document".to_string()),
            meta_id: Some("doc1".to_string()),
            meta: Some(meta),
            wallet_info: Some(wallet_info),
            ..Default::default()
        };
        
        let atom = Atom::create(params);
        
        assert_eq!(atom.isotope, Isotope::M);
        assert_eq!(atom.value, Some("100".to_string()));
        assert_eq!(atom.position, "wallet_pos");
        assert_eq!(atom.wallet_address, "wallet_addr");
        assert_eq!(atom.token, "WALLET_TOKEN");
        assert_eq!(atom.batch_id, Some("batch123".to_string()));
        assert_eq!(atom.meta_type, Some("document".to_string()));
        assert_eq!(atom.meta.len(), 1);
    }
    
    #[test]
    fn test_hashable_props() {
        let props = Atom::get_hashable_props();
        assert!(props.contains(&"position"));
        assert!(props.contains(&"walletAddress"));
        assert!(props.contains(&"isotope"));
        assert!(props.contains(&"token"));
        assert!(props.contains(&"meta"));
        assert!(!props.contains(&"otsFragment"));
        assert!(!props.contains(&"index"));
    }
    
    #[test]
    fn test_unclaimed_props() {
        let props = Atom::get_unclaimed_props();
        assert!(props.contains(&"otsFragment"));
    }
    
    #[test]
    fn test_get_hashable_values() {
        let mut atom = Atom::new("pos123", "addr456", Isotope::V, "TEST");
        atom.value = Some("100".to_string());
        atom.meta = vec![MetaItem::new("key1", "value1")];
        
        let values = atom.get_hashable_values();
        
        // Should contain position, walletAddress, isotope, token, value, createdAt, and meta key-value pairs
        assert!(values.contains(&"pos123".to_string()));
        assert!(values.contains(&"addr456".to_string()));
        assert!(values.contains(&"V".to_string()));
        assert!(values.contains(&"TEST".to_string()));
        assert!(values.contains(&"100".to_string()));
        assert!(values.contains(&"key1".to_string()));
        assert!(values.contains(&"value1".to_string()));
    }
    
    #[test]
    fn test_sort_atoms() {
        let mut atom1 = Atom::new("pos1", "addr1", Isotope::V, "TEST");
        atom1.index = Some(2);
        
        let mut atom2 = Atom::new("pos2", "addr2", Isotope::V, "TEST");
        atom2.index = Some(1);
        
        let mut atom3 = Atom::new("pos3", "addr3", Isotope::V, "TEST");
        atom3.index = Some(3);
        
        let atoms = vec![atom1, atom2, atom3];
        let sorted = Atom::sort_atoms(&atoms);
        
        assert_eq!(sorted[0].index, Some(1));
        assert_eq!(sorted[1].index, Some(2));
        assert_eq!(sorted[2].index, Some(3));
    }
    
    #[test]
    fn test_aggregated_meta() {
        let mut atom = Atom::new("pos", "addr", Isotope::M, "TEST");
        atom.meta = vec![
            MetaItem::new("key1", "value1"),
            MetaItem::new("key2", "value2"),
        ];
        
        let aggregated = atom.aggregated_meta();
        assert_eq!(aggregated.get("key1"), Some(&"value1".to_string()));
        assert_eq!(aggregated.get("key2"), Some(&"value2".to_string()));
    }
    
    #[test]
    fn test_json_serialization() {
        let mut atom = Atom::new("pos", "addr", Isotope::V, "TEST");
        atom.ots_fragment = Some("should_be_excluded".to_string());
        
        let json = atom.toJSON().unwrap();
        
        // Should contain regular properties but exclude otsFragment
        assert!(json.contains("\"position\":\"pos\""));
        assert!(json.contains("\"isotope\":\"V\""));
        assert!(!json.contains("otsFragment"));
        assert!(!json.contains("should_be_excluded"));
    }
    
    #[test]
    fn test_json_deserialization() {
        let json = r#"{
            "position": "test_pos",
            "walletAddress": "test_addr",
            "isotope": "V",
            "token": "TEST",
            "value": "100",
            "meta": [],
            "createdAt": "1640995200000"
        }"#;
        
        let atom = Atom::json_to_object(json).unwrap();
        assert_eq!(atom.position, "test_pos");
        assert_eq!(atom.wallet_address, "test_addr");
        assert_eq!(atom.isotope, Isotope::V);
        assert_eq!(atom.token, "TEST");
        assert_eq!(atom.value, Some("100".to_string()));
    }
    
    #[test]
    fn test_hash_atoms_empty() {
        let atoms: Vec<Atom> = vec![];
        let result = Atom::hash_atoms(&atoms, "hex");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnishIOError::AtomsMissing));
    }
    
    #[test]
    fn test_hash_atoms_basic() {
        let mut atom1 = Atom::new("pos1", "addr1", Isotope::V, "TEST");
        atom1.index = Some(1);
        atom1.value = Some("100".to_string());
        
        let mut atom2 = Atom::new("pos2", "addr2", Isotope::V, "TEST");
        atom2.index = Some(2);
        atom2.value = Some("200".to_string());
        
        let atoms = vec![atom1, atom2];
        let hash_hex = Atom::hash_atoms(&atoms, "hex").unwrap();
        let hash_base17 = Atom::hash_atoms(&atoms, "base17").unwrap();
        
        // Hash should be 64 characters hex
        assert_eq!(hash_hex.len(), 64);
        assert!(hash_hex.chars().all(|c| c.is_ascii_hexdigit()));
        
        // Base17 should be 64 characters
        assert_eq!(hash_base17.len(), 64);
    }
    
    #[test]
    fn test_hex_to_base17() {
        let hex = "0123456789abcdef";
        let base17 = hex_to_base17(hex);
        assert_eq!(base17.len(), 64);
        
        // Test with zero
        let zero_base17 = hex_to_base17("0");
        assert_eq!(zero_base17, "0".repeat(64));
    }
}

// JavaScript-style convenience methods for cross-SDK validation
impl Atom {
    /// Rust-style method (satisfies compiler warnings)
    pub fn to_json_string(&self) -> crate::error::Result<String> {
        self.toJSON()
    }
    
    /// Rust-style method (satisfies compiler warnings)
    pub fn from_json_string(json: &str) -> crate::error::Result<Self> {
        Self::fromJSON(json)
    }
    
    /// JavaScript-style toJSON() convenience method
    /// Returns JSON string directly, matching JavaScript SDK pattern  
    #[allow(non_snake_case)]
    pub fn toJSON(&self) -> crate::error::Result<String> {
        let options = crate::types::AtomJsonOptions::default();
        let json_value = self.to_json(options)?;
        serde_json::to_string(&json_value)
            .map_err(|e| crate::error::KnishIOError::custom(&format!("JSON serialization failed: {}", e)))
    }
    
    /// JavaScript-style fromJSON() convenience method
    /// Creates Atom instance from JSON string, matching JavaScript SDK pattern
    #[allow(non_snake_case)]
    pub fn fromJSON(json: &str) -> crate::error::Result<Self> {
        let json_value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| crate::error::KnishIOError::custom(&format!("JSON parsing failed: {}", e)))?;
        let options = crate::types::AtomFromJsonOptions::default();
        Self::from_json(&json_value, options)
    }
}

