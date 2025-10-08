//! Token Unit module for the KnishIO SDK
//!
//! This module provides the TokenUnit struct and associated methods for managing
//! NFT and stackable token units, ensuring exact compatibility with the JavaScript
//! TokenUnit.js implementation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{KnishIOError, Result};

/// Represents a token unit with its metadata
///
/// TokenUnit manages individual token instances, including NFTs and stackable tokens.
/// This struct maintains exact compatibility with the JavaScript TokenUnit class.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenUnit {
    /// Unique identifier for this token unit
    pub id: String,
    
    /// Human-readable name of the token unit
    pub name: String,
    
    /// Metadata associated with this token unit
    pub metas: HashMap<String, serde_json::Value>,
}

impl TokenUnit {
    /// Create a new TokenUnit instance
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the token unit
    /// * `name` - Human-readable name
    /// * `metas` - Optional metadata map
    ///
    /// # Returns
    ///
    /// New TokenUnit instance
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use std::collections::HashMap;
    ///
    /// let token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     Some(HashMap::new())
    /// );
    /// ```
    pub fn new(id: String, name: String, metas: Option<HashMap<String, serde_json::Value>>) -> Self {
        Self {
            id,
            name,
            metas: metas.unwrap_or_else(HashMap::new),
        }
    }

    /// Create a TokenUnit from GraphQL response data
    ///
    /// Equivalent to TokenUnit.createFromGraphQL() in JavaScript SDK
    ///
    /// # Arguments
    ///
    /// * `data` - GraphQL response data containing id, name, and metas
    ///
    /// # Returns
    ///
    /// Result containing the new TokenUnit or an error
    ///
    /// # Example
    ///
    /// ```rust
    /// use serde_json::json;
    /// use knishio_client::token_unit::TokenUnit;
    ///
    /// let data = json!({
    ///     "id": "token123",
    ///     "name": "My Token",
    ///     "metas": "{\"fragmentZone\": \"zone1\"}"
    /// });
    ///
    /// let token_unit = TokenUnit::create_from_graphql(&data).unwrap();
    /// ```
    pub fn create_from_graphql(data: &serde_json::Value) -> Result<Self> {
        let id = data.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KnishIOError::custom("Missing 'id' field in GraphQL data"))?
            .to_string();

        let name = data.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KnishIOError::custom("Missing 'name' field in GraphQL data"))?
            .to_string();

        let metas = if let Some(metas_value) = data.get("metas") {
            if let Some(metas_str) = metas_value.as_str() {
                // Parse JSON string if it's a string
                if !metas_str.is_empty() {
                    match serde_json::from_str::<HashMap<String, serde_json::Value>>(metas_str) {
                        Ok(parsed_metas) => parsed_metas,
                        Err(_) => {
                            // If parsing fails, create empty map instead
                            HashMap::new()
                        }
                    }
                } else {
                    HashMap::new()
                }
            } else if let Some(metas_obj) = metas_value.as_object() {
                // Convert serde_json::Map to HashMap
                metas_obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        Ok(Self { id, name, metas })
    }

    /// Create a TokenUnit from database array data
    ///
    /// Equivalent to TokenUnit.createFromDB() in JavaScript SDK
    ///
    /// # Arguments
    ///
    /// * `data` - Array-like data [id, name, metas?]
    ///
    /// # Returns
    ///
    /// Result containing the new TokenUnit or an error
    ///
    /// # Example
    ///
    /// ```rust
    /// use serde_json::json;
    /// use knishio_client::token_unit::TokenUnit;
    ///
    /// let data = json!(["token123", "My Token", {"fragmentZone": "zone1"}]);
    /// let token_unit = TokenUnit::create_from_db(&data).unwrap();
    /// ```
    pub fn create_from_db(data: &serde_json::Value) -> Result<Self> {
        let array = data.as_array()
            .ok_or_else(|| KnishIOError::custom("Expected array data for TokenUnit"))?;

        if array.len() < 2 {
            return Err(KnishIOError::custom("Array must contain at least id and name"));
        }

        let id = array[0].as_str()
            .ok_or_else(|| KnishIOError::custom("First element (id) must be a string"))?
            .to_string();

        let name = array[1].as_str()
            .ok_or_else(|| KnishIOError::custom("Second element (name) must be a string"))?
            .to_string();

        let metas = if array.len() > 2 {
            if let Some(metas_obj) = array[2].as_object() {
                metas_obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        Ok(Self { id, name, metas })
    }

    /// Get the fragment zone from metadata
    ///
    /// Equivalent to getFragmentZone() in JavaScript SDK
    ///
    /// # Returns
    ///
    /// Optional fragment zone string
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let mut metas = HashMap::new();
    /// metas.insert("fragmentZone".to_string(), json!("zone1"));
    ///
    /// let token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     Some(metas)
    /// );
    ///
    /// assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
    /// ```
    pub fn get_fragment_zone(&self) -> Option<String> {
        self.metas.get("fragmentZone")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get the fused token units from metadata
    ///
    /// Equivalent to getFusedTokenUnits() in JavaScript SDK
    ///
    /// # Returns
    ///
    /// Optional fused token units data
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let mut metas = HashMap::new();
    /// metas.insert("fusedTokenUnits".to_string(), json!(["unit1", "unit2"]));
    ///
    /// let token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     Some(metas)
    /// );
    ///
    /// let fused_units = token_unit.get_fused_token_units().unwrap();
    /// assert_eq!(fused_units, json!(["unit1", "unit2"]));
    /// ```
    pub fn get_fused_token_units(&self) -> Option<serde_json::Value> {
        self.metas.get("fusedTokenUnits").cloned()
    }

    /// Convert TokenUnit to array data format
    ///
    /// Equivalent to toData() in JavaScript SDK
    ///
    /// # Returns
    ///
    /// Array representation [id, name, metas]
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use serde_json::json;
    ///
    /// let token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// let data = token_unit.to_data();
    /// assert_eq!(data[0], json!("token123"));
    /// assert_eq!(data[1], json!("My Token"));
    /// ```
    pub fn to_data(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::Value::String(self.id.clone()),
            serde_json::Value::String(self.name.clone()),
            serde_json::to_value(&self.metas).unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
        ]
    }

    /// Convert TokenUnit to GraphQL response format
    ///
    /// Equivalent to toGraphQLResponse() in JavaScript SDK
    ///
    /// # Returns
    ///
    /// GraphQL-compatible response object
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use serde_json::json;
    ///
    /// let token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// let response = token_unit.to_graphql_response();
    /// assert_eq!(response["id"], json!("token123"));
    /// assert_eq!(response["name"], json!("My Token"));
    /// ```
    pub fn to_graphql_response(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "metas": serde_json::to_string(&self.metas).unwrap_or_else(|_| "{}".to_string())
        })
    }

    /// Set a metadata value
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    /// * `value` - Metadata value
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use serde_json::json;
    ///
    /// let mut token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// token_unit.set_meta("fragmentZone", json!("zone1"));
    /// assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
    /// ```
    pub fn set_meta(&mut self, key: &str, value: serde_json::Value) {
        self.metas.insert(key.to_string(), value);
    }

    /// Get a metadata value
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    ///
    /// # Returns
    ///
    /// Optional metadata value
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use serde_json::json;
    ///
    /// let mut token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// token_unit.set_meta("fragmentZone", json!("zone1"));
    /// assert_eq!(token_unit.get_meta("fragmentZone"), Some(&json!("zone1")));
    /// ```
    pub fn get_meta(&self, key: &str) -> Option<&serde_json::Value> {
        self.metas.get(key)
    }

    /// Remove a metadata value
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to remove
    ///
    /// # Returns
    ///
    /// Previously stored value, if any
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use serde_json::json;
    ///
    /// let mut token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// token_unit.set_meta("fragmentZone", json!("zone1"));
    /// let removed = token_unit.remove_meta("fragmentZone");
    /// assert_eq!(removed, Some(json!("zone1")));
    /// assert_eq!(token_unit.get_fragment_zone(), None);
    /// ```
    pub fn remove_meta(&mut self, key: &str) -> Option<serde_json::Value> {
        self.metas.remove(key)
    }

    /// Check if this token unit has any metadata
    ///
    /// # Returns
    ///
    /// True if metadata is not empty
    pub fn has_metadata(&self) -> bool {
        !self.metas.is_empty()
    }

    /// Get all metadata keys
    ///
    /// # Returns
    ///
    /// Vector of all metadata keys
    pub fn metadata_keys(&self) -> Vec<String> {
        self.metas.keys().cloned().collect()
    }

    /// Clear all metadata
    pub fn clear_metadata(&mut self) {
        self.metas.clear();
    }
}

impl Default for TokenUnit {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            metas: HashMap::new(),
        }
    }
}

impl std::fmt::Display for TokenUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TokenUnit(id: {}, name: {})", self.id, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_token_unit_new() {
        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            None,
        );

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert!(token_unit.metas.is_empty());
    }

    #[test]
    fn test_token_unit_new_with_metas() {
        let mut metas = HashMap::new();
        metas.insert("fragmentZone".to_string(), json!("zone1"));
        metas.insert("fusedTokenUnits".to_string(), json!(["unit1", "unit2"]));

        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas),
        );

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert_eq!(token_unit.metas.len(), 2);
        assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
    }

    #[test]
    fn test_create_from_graphql() {
        let data = json!({
            "id": "token123",
            "name": "My Token",
            "metas": "{\"fragmentZone\": \"zone1\", \"fusedTokenUnits\": [\"unit1\", \"unit2\"]}"
        });

        let token_unit = TokenUnit::create_from_graphql(&data).unwrap();

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
        assert_eq!(token_unit.get_fused_token_units(), Some(json!(["unit1", "unit2"])));
    }

    #[test]
    fn test_create_from_graphql_empty_metas() {
        let data = json!({
            "id": "token123",
            "name": "My Token",
            "metas": ""
        });

        let token_unit = TokenUnit::create_from_graphql(&data).unwrap();

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert!(token_unit.metas.is_empty());
    }

    #[test]
    fn test_create_from_graphql_object_metas() {
        let data = json!({
            "id": "token123",
            "name": "My Token",
            "metas": {
                "fragmentZone": "zone1",
                "fusedTokenUnits": ["unit1", "unit2"]
            }
        });

        let token_unit = TokenUnit::create_from_graphql(&data).unwrap();

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
        assert_eq!(token_unit.get_fused_token_units(), Some(json!(["unit1", "unit2"])));
    }

    #[test]
    fn test_create_from_db() {
        let data = json!([
            "token123",
            "My Token",
            {
                "fragmentZone": "zone1",
                "fusedTokenUnits": ["unit1", "unit2"]
            }
        ]);

        let token_unit = TokenUnit::create_from_db(&data).unwrap();

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
        assert_eq!(token_unit.get_fused_token_units(), Some(json!(["unit1", "unit2"])));
    }

    #[test]
    fn test_create_from_db_minimal() {
        let data = json!(["token123", "My Token"]);

        let token_unit = TokenUnit::create_from_db(&data).unwrap();

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert!(token_unit.metas.is_empty());
    }

    #[test]
    fn test_get_fragment_zone() {
        let mut metas = HashMap::new();
        metas.insert("fragmentZone".to_string(), json!("zone1"));

        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas),
        );

        assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));

        let token_unit_empty = TokenUnit::new(
            "token456".to_string(),
            "Empty Token".to_string(),
            None,
        );

        assert_eq!(token_unit_empty.get_fragment_zone(), None);
    }

    #[test]
    fn test_get_fused_token_units() {
        let mut metas = HashMap::new();
        metas.insert("fusedTokenUnits".to_string(), json!(["unit1", "unit2"]));

        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas),
        );

        assert_eq!(token_unit.get_fused_token_units(), Some(json!(["unit1", "unit2"])));

        let token_unit_empty = TokenUnit::new(
            "token456".to_string(),
            "Empty Token".to_string(),
            None,
        );

        assert_eq!(token_unit_empty.get_fused_token_units(), None);
    }

    #[test]
    fn test_to_data() {
        let mut metas = HashMap::new();
        metas.insert("fragmentZone".to_string(), json!("zone1"));

        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas),
        );

        let data = token_unit.to_data();

        assert_eq!(data.len(), 3);
        assert_eq!(data[0], json!("token123"));
        assert_eq!(data[1], json!("My Token"));
        
        // Check that metas is properly serialized
        let metas_value = &data[2];
        assert!(metas_value.is_object());
        assert_eq!(metas_value["fragmentZone"], json!("zone1"));
    }

    #[test]
    fn test_to_graphql_response() {
        let mut metas = HashMap::new();
        metas.insert("fragmentZone".to_string(), json!("zone1"));

        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas),
        );

        let response = token_unit.to_graphql_response();

        assert_eq!(response["id"], json!("token123"));
        assert_eq!(response["name"], json!("My Token"));
        
        // Check that metas is properly JSON-encoded
        let metas_str = response["metas"].as_str().unwrap();
        let parsed_metas: HashMap<String, serde_json::Value> = 
            serde_json::from_str(metas_str).unwrap();
        assert_eq!(parsed_metas["fragmentZone"], json!("zone1"));
    }

    #[test]
    fn test_metadata_operations() {
        let mut token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            None,
        );

        assert!(!token_unit.has_metadata());
        assert!(token_unit.metadata_keys().is_empty());

        // Set metadata
        token_unit.set_meta("key1", json!("value1"));
        token_unit.set_meta("key2", json!(42));

        assert!(token_unit.has_metadata());
        assert_eq!(token_unit.metadata_keys().len(), 2);
        assert!(token_unit.metadata_keys().contains(&"key1".to_string()));
        assert!(token_unit.metadata_keys().contains(&"key2".to_string()));

        // Get metadata
        assert_eq!(token_unit.get_meta("key1"), Some(&json!("value1")));
        assert_eq!(token_unit.get_meta("key2"), Some(&json!(42)));
        assert_eq!(token_unit.get_meta("nonexistent"), None);

        // Remove metadata
        let removed = token_unit.remove_meta("key1");
        assert_eq!(removed, Some(json!("value1")));
        assert_eq!(token_unit.get_meta("key1"), None);
        assert_eq!(token_unit.metadata_keys().len(), 1);

        // Clear all metadata
        token_unit.clear_metadata();
        assert!(!token_unit.has_metadata());
        assert!(token_unit.metadata_keys().is_empty());
    }

    #[test]
    fn test_error_handling() {
        // Test missing required fields
        let invalid_data = json!({
            "name": "My Token"
            // Missing "id"
        });

        let result = TokenUnit::create_from_graphql(&invalid_data);
        assert!(result.is_err());

        // Test invalid array data
        let invalid_array = json!(["only_one_element"]);
        let result = TokenUnit::create_from_db(&invalid_array);
        assert!(result.is_err());

        // Test non-array data
        let invalid_data = json!({"not": "array"});
        let result = TokenUnit::create_from_db(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization() {
        let mut metas = HashMap::new();
        metas.insert("fragmentZone".to_string(), json!("zone1"));

        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas),
        );

        // Test JSON serialization
        let json_str = serde_json::to_string(&token_unit).unwrap();
        let deserialized: TokenUnit = serde_json::from_str(&json_str).unwrap();

        assert_eq!(token_unit, deserialized);
        assert_eq!(token_unit.get_fragment_zone(), deserialized.get_fragment_zone());
    }

    #[test]
    fn test_display() {
        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            None,
        );

        let display_str = format!("{}", token_unit);
        assert_eq!(display_str, "TokenUnit(id: token123, name: My Token)");
    }

    #[test]
    fn test_default() {
        let token_unit = TokenUnit::default();
        
        assert!(token_unit.id.is_empty());
        assert!(token_unit.name.is_empty());
        assert!(token_unit.metas.is_empty());
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test exact JavaScript SDK behavior reproduction

        // Test createFromGraphQL with length check (JavaScript line 72-76)
        let data_with_empty_metas = json!({
            "id": "token123",
            "name": "My Token",
            "metas": ""
        });

        let token_unit = TokenUnit::create_from_graphql(&data_with_empty_metas).unwrap();
        assert!(token_unit.metas.is_empty());

        // Test createFromDB with optional third element (JavaScript line 90-96)
        let minimal_data = json!(["token123", "My Token"]);
        let token_unit = TokenUnit::create_from_db(&minimal_data).unwrap();
        assert!(token_unit.metas.is_empty());

        let full_data = json!(["token123", "My Token", {"key": "value"}]);
        let token_unit = TokenUnit::create_from_db(&full_data).unwrap();
        assert_eq!(token_unit.metas.len(), 1);
        assert_eq!(token_unit.get_meta("key"), Some(&json!("value")));

        // Test null handling for getFragmentZone (JavaScript line 103)
        let token_unit_no_zone = TokenUnit::new("test".to_string(), "test".to_string(), None);
        assert_eq!(token_unit_no_zone.get_fragment_zone(), None);

        // Test null handling for getFusedTokenUnits (JavaScript line 110)
        assert_eq!(token_unit_no_zone.get_fused_token_units(), None);
    }
}