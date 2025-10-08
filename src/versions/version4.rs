//! Version4 implementation
//!
//! Equivalent to Version4.js in the JavaScript SDK

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::atom::Atom;
use crate::types::{Isotope, MetaItem};
use super::{HashAtom, AtomVersion};

/// Version 4 atom implementation
///
/// Equivalent to Version4 class in JavaScript SDK
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Version4 {
    /// Wallet position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    
    /// Wallet address
    #[serde(rename = "walletAddress")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    
    /// Atom isotope (transaction type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isotope: Option<Isotope>,
    
    /// Token slug
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    
    /// Value (for value transfer atoms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    
    /// Batch ID
    #[serde(rename = "batchId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    
    /// Metadata type
    #[serde(rename = "metaType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_type: Option<String>,
    
    /// Metadata ID
    #[serde(rename = "metaId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_id: Option<String>,
    
    /// Metadata items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Vec<MetaItem>>,
    
    /// Atom index within molecule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    
    /// Version identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl Version4 {
    /// Create a new Version4 instance
    ///
    /// Equivalent to Version4 constructor in JavaScript
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        position: Option<String>,
        wallet_address: Option<String>,
        isotope: Option<Isotope>,
        token: Option<String>,
        value: Option<f64>,
        batch_id: Option<String>,
        meta_type: Option<String>,
        meta_id: Option<String>,
        meta: Option<Vec<MetaItem>>,
        index: Option<i32>,
        created_at: Option<i64>,
        version: Option<String>,
    ) -> Self {
        Self {
            position,
            wallet_address,
            isotope,
            token,
            value,
            batch_id,
            meta_type,
            meta_id,
            meta,
            index,
            created_at,
            version,
        }
    }

    /// Create a Version4 from an Atom
    ///
    /// Equivalent to Version4.create() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `atom` - The atom to create from
    ///
    /// # Returns
    ///
    /// A new Version4 instance with the atom's data
    pub fn from_atom(atom: &Atom) -> Self {
        Self {
            position: Some(atom.position.clone()),
            wallet_address: Some(atom.wallet_address.clone()),
            isotope: Some(atom.isotope.clone()),
            token: Some(atom.token.clone()),
            // Convert Option<String> to Option<f64>
            value: atom.value.as_ref().and_then(|v| v.parse::<f64>().ok()),
            batch_id: atom.batch_id.clone(),
            meta_type: atom.meta_type.clone(),
            meta_id: atom.meta_id.clone(),
            // Convert Vec<MetaItem> to Option<Vec<MetaItem>>
            meta: Some(atom.meta.clone()),
            // Convert Option<u32> to Option<i32>
            index: atom.index.map(|i| i as i32),
            // Convert String to Option<i64> (parse timestamp)
            created_at: atom.created_at.parse::<i64>().ok(),
            version: atom.version.clone(),
        }
    }

    /// Get the structured view of this Version4 atom
    ///
    /// Equivalent to view() method inherited from HashAtom in JavaScript
    ///
    /// # Returns
    ///
    /// Structured representation for hashing
    pub fn view(&self) -> Value {
        let value = serde_json::to_value(self).unwrap_or(Value::Null);
        HashAtom::structure(&value)
    }

    /// Convert to a generic Atom
    ///
    /// Helper method to convert back to the base Atom type
    pub fn to_atom(&self) -> Atom {
        let mut atom = Atom::new(
            self.position.as_deref().unwrap_or(""),
            self.wallet_address.as_deref().unwrap_or(""),
            self.isotope.clone().unwrap_or(Isotope::V),
            self.token.as_deref().unwrap_or(""),
        );

        // Convert Option<f64> to Option<String>
        atom.value = self.value.map(|v| v.to_string());
        atom.batch_id = self.batch_id.clone();
        atom.meta_type = self.meta_type.clone();
        atom.meta_id = self.meta_id.clone();
        // Convert Option<Vec<MetaItem>> to Vec<MetaItem>
        atom.meta = self.meta.clone().unwrap_or_default();
        // Convert Option<i32> to Option<u32>
        atom.index = self.index.map(|i| i as u32);
        // Convert Option<i64> to String
        atom.created_at = self.created_at.map(|t| t.to_string()).unwrap_or_else(|| chrono::Utc::now().timestamp_millis().to_string());
        atom.version = self.version.clone();

        atom
    }

    /// Check if this Version4 atom has all required fields
    pub fn is_valid(&self) -> bool {
        self.position.is_some() 
            && self.wallet_address.is_some() 
            && self.isotope.is_some() 
            && self.token.is_some()
    }

    /// Get a hash-friendly representation
    ///
    /// This removes None values and ensures consistent ordering
    pub fn hash_representation(&self) -> Value {
        // Create a copy with only non-None values
        let mut map = serde_json::Map::new();

        if let Some(ref position) = self.position {
            map.insert("position".to_string(), Value::String(position.clone()));
        }
        if let Some(ref wallet_address) = self.wallet_address {
            map.insert("walletAddress".to_string(), Value::String(wallet_address.clone()));
        }
        if let Some(ref isotope) = self.isotope {
            map.insert("isotope".to_string(), serde_json::to_value(isotope).unwrap_or(Value::Null));
        }
        if let Some(ref token) = self.token {
            map.insert("token".to_string(), Value::String(token.clone()));
        }
        if let Some(value) = self.value {
            map.insert("value".to_string(), Value::Number(serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0))));
        }
        if let Some(ref batch_id) = self.batch_id {
            map.insert("batchId".to_string(), Value::String(batch_id.clone()));
        }
        if let Some(ref meta_type) = self.meta_type {
            map.insert("metaType".to_string(), Value::String(meta_type.clone()));
        }
        if let Some(ref meta_id) = self.meta_id {
            map.insert("metaId".to_string(), Value::String(meta_id.clone()));
        }
        if let Some(ref meta) = self.meta {
            map.insert("meta".to_string(), serde_json::to_value(meta).unwrap_or(Value::Null));
        }
        if let Some(index) = self.index {
            map.insert("index".to_string(), Value::Number(index.into()));
        }
        if let Some(created_at) = self.created_at {
            map.insert("createdAt".to_string(), Value::Number(created_at.into()));
        }
        if let Some(ref version) = self.version {
            map.insert("version".to_string(), Value::String(version.clone()));
        }

        let object = Value::Object(map);
        HashAtom::structure(&object)
    }
}

impl Default for Version4 {
    fn default() -> Self {
        Self::new(
            None, None, None, None, None, None,
            None, None, None, None, None, None,
        )
    }
}

impl AtomVersion for Version4 {
    fn from_atom(atom: &Atom) -> Self {
        Self::from_atom(atom)
    }
    
    fn view(&self) -> Value {
        self.view()
    }
    
    fn version(&self) -> String {
        "4".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_version4_new() {
        let version4 = Version4::new(
            Some("W1".to_string()),
            Some("test-address".to_string()),
            Some(Isotope::V),
            Some("TEST".to_string()),
            Some(100.0),
            Some("batch123".to_string()),
            Some("user".to_string()),
            Some("id123".to_string()),
            None,
            Some(0),
            Some(1234567890),
            Some("4".to_string()),
        );

        assert_eq!(version4.position, Some("W1".to_string()));
        assert_eq!(version4.wallet_address, Some("test-address".to_string()));
        assert_eq!(version4.isotope, Some(Isotope::V));
        assert_eq!(version4.token, Some("TEST".to_string()));
        assert_eq!(version4.value, Some(100.0));
        assert_eq!(version4.batch_id, Some("batch123".to_string()));
        assert_eq!(version4.meta_type, Some("user".to_string()));
        assert_eq!(version4.meta_id, Some("id123".to_string()));
        assert_eq!(version4.index, Some(0));
        assert_eq!(version4.created_at, Some(1234567890));
        assert_eq!(version4.version, Some("4".to_string()));
    }

    #[test]
    fn test_version4_from_atom() {
        let mut atom = Atom::new(
            "W1",
            "test-address",
            Isotope::C,
            "TEST"
        );
        atom.value = Some(50.0);
        atom.batch_id = Some("batch456".to_string());
        atom.meta_type = Some("identity".to_string());
        atom.meta_id = Some("id456".to_string());
        atom.index = Some(1);
        atom.created_at = Some(9876543210);
        atom.version = Some("4".to_string());

        let version4 = Version4::from_atom(&atom);

        assert_eq!(version4.position, Some("W1".to_string()));
        assert_eq!(version4.wallet_address, Some("test-address".to_string()));
        assert_eq!(version4.isotope, Some(Isotope::C));
        assert_eq!(version4.token, Some("TEST".to_string()));
        assert_eq!(version4.value, Some(50.0));
        assert_eq!(version4.batch_id, Some("batch456".to_string()));
        assert_eq!(version4.meta_type, Some("identity".to_string()));
        assert_eq!(version4.meta_id, Some("id456".to_string()));
        assert_eq!(version4.index, Some(1));
        assert_eq!(version4.created_at, Some(9876543210));
        assert_eq!(version4.version, Some("4".to_string()));
    }

    #[test]
    fn test_version4_to_atom() {
        let version4 = Version4::new(
            Some("W2".to_string()),
            Some("addr2".to_string()),
            Some(Isotope::M),
            Some("TOKEN2".to_string()),
            Some(200.0),
            None,
            None,
            None,
            None,
            Some(2),
            Some(1111111111),
            Some("4".to_string()),
        );

        let atom = version4.to_atom();

        assert_eq!(atom.position, "W2");
        assert_eq!(atom.wallet_address, "addr2");
        assert_eq!(atom.isotope, Isotope::M);
        assert_eq!(atom.token, "TOKEN2");
        assert_eq!(atom.value, Some(200.0));
        assert_eq!(atom.batch_id, None);
        assert_eq!(atom.meta_type, None);
        assert_eq!(atom.meta_id, None);
        assert_eq!(atom.index, Some(2));
        assert_eq!(atom.created_at, Some(1111111111));
        assert_eq!(atom.version, Some("4".to_string()));
    }

    #[test]
    fn test_version4_view() {
        let version4 = Version4::new(
            Some("W1".to_string()),
            Some("test-address".to_string()),
            Some(Isotope::V),
            Some("TEST".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        let view = version4.view();

        // Should return structured representation
        assert!(view.is_array() || view.is_object());

        // If it's an array, it should contain structured key-value pairs
        if let Some(arr) = view.as_array() {
            assert!(!arr.is_empty());
            // Each item should be an object with a single key-value pair
            for item in arr {
                assert!(item.is_object());
                assert_eq!(item.as_object().unwrap().len(), 1);
            }
        }
    }

    #[test]
    fn test_version4_is_valid() {
        // Valid version4 (all required fields)
        let valid = Version4::new(
            Some("W1".to_string()),
            Some("address".to_string()),
            Some(Isotope::V),
            Some("TOKEN".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(valid.is_valid());

        // Invalid version4 (missing required fields)
        let invalid = Version4::new(
            None, // Missing position
            Some("address".to_string()),
            Some(Isotope::V),
            Some("TOKEN".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(!invalid.is_valid());

        // Invalid version4 (missing wallet address)
        let invalid2 = Version4::new(
            Some("W1".to_string()),
            None, // Missing wallet address
            Some(Isotope::V),
            Some("TOKEN".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(!invalid2.is_valid());
    }

    #[test]
    fn test_version4_hash_representation() {
        let version4 = Version4::new(
            Some("W1".to_string()),
            Some("test-address".to_string()),
            Some(Isotope::V),
            Some("TEST".to_string()),
            Some(100.0),
            Some("batch123".to_string()),
            None, // Skip this field
            None, // Skip this field
            None,
            Some(0),
            None, // Skip this field
            Some("4".to_string()),
        );

        let hash_repr = version4.hash_representation();

        // Should be structured array
        assert!(hash_repr.is_array());
        
        let arr = hash_repr.as_array().unwrap();
        
        // Should contain only non-None fields, sorted by key
        // Expected fields: batchId, index, isotope, position, token, value, version, walletAddress
        assert_eq!(arr.len(), 8);
        
        // Verify key ordering (alphabetical)
        let keys: Vec<String> = arr.iter()
            .map(|item| item.as_object().unwrap().keys().next().unwrap().clone())
            .collect();
        
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys);
    }

    #[test]
    fn test_version4_serialization() {
        let version4 = Version4::new(
            Some("W1".to_string()),
            Some("test-address".to_string()),
            Some(Isotope::V),
            Some("TEST".to_string()),
            Some(100.0),
            None,
            None,
            None,
            None,
            Some(0),
            Some(1234567890),
            Some("4".to_string()),
        );

        // Test JSON serialization
        let json_str = serde_json::to_string(&version4).unwrap();
        let deserialized: Version4 = serde_json::from_str(&json_str).unwrap();

        assert_eq!(version4, deserialized);
    }

    #[test]
    fn test_atom_version_trait() {
        let atom = Atom::new("W1", "addr", Isotope::V, "TOKEN");
        
        let version4 = Version4::from_atom(&atom);
        assert_eq!(version4.version(), "4");
        
        let view = version4.view();
        assert!(view.is_array() || view.is_object());
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test that our implementation matches JavaScript SDK patterns
        
        // Test constructor with same parameters as JS
        let version4 = Version4::new(
            Some("W1".to_string()),
            Some("walletAddress".to_string()),
            Some(Isotope::V),
            Some("TOKEN".to_string()),
            Some(100.0),
            Some("batchId".to_string()),
            Some("metaType".to_string()),
            Some("metaId".to_string()),
            None, // meta
            Some(0), // index
            Some(1234567890), // createdAt
            Some("4".to_string()), // version
        );
        
        // Test serialization matches JS field names
        let json_value = serde_json::to_value(&version4).unwrap();
        
        assert_eq!(json_value["position"], "W1");
        assert_eq!(json_value["walletAddress"], "walletAddress"); // camelCase
        assert_eq!(json_value["isotope"], "V");
        assert_eq!(json_value["token"], "TOKEN");
        assert_eq!(json_value["value"], 100.0);
        assert_eq!(json_value["batchId"], "batchId"); // camelCase
        assert_eq!(json_value["metaType"], "metaType"); // camelCase
        assert_eq!(json_value["metaId"], "metaId"); // camelCase
        assert_eq!(json_value["index"], 0);
        assert_eq!(json_value["createdAt"], 1234567890); // camelCase
        assert_eq!(json_value["version"], "4");
        
        // Test create method (equivalent to JS Version4.create)
        let atom = Atom::new("W2", "addr2", Isotope::C, "TOKEN2");
        let version4_from_atom = Version4::from_atom(&atom);
        
        assert_eq!(version4_from_atom.position, Some("W2".to_string()));
        assert_eq!(version4_from_atom.wallet_address, Some("addr2".to_string()));
        assert_eq!(version4_from_atom.isotope, Some(Isotope::C));
        assert_eq!(version4_from_atom.token, Some("TOKEN2".to_string()));
        
        // Test view method (inherited from HashAtom)
        let view = version4.view();
        assert!(view.is_array()); // Should be structured as array
        
        // Test that view is sorted properly
        if let Some(arr) = view.as_array() {
            let keys: Vec<String> = arr.iter()
                .map(|item| item.as_object().unwrap().keys().next().unwrap().clone())
                .collect();
            
            let mut sorted_keys = keys.clone();
            sorted_keys.sort();
            assert_eq!(keys, sorted_keys); // Keys should be pre-sorted
        }
    }
}