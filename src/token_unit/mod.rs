//! Token Unit module for the KnishIO SDK
//!
//! This module provides the TokenUnit struct and associated methods for managing
//! NFT and stackable token units, ensuring exact compatibility with the JavaScript
//! TokenUnit.js implementation.

use crate::error::{KnishIOError, Result};
use crate::types::ordered::{serialize_js_object, OrderedMap, OrderedValue};
use serde::{Deserialize, Serialize};

/// Represents a token unit with its metadata
///
/// TokenUnit manages individual token instances, including NFTs and stackable tokens.
/// This struct maintains exact compatibility with the JavaScript TokenUnit class.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct TokenUnit {
    /// Unique identifier for this token unit
    pub id: String,

    /// Human-readable name of the token unit
    pub name: String,

    /// Metadata associated with this token unit, in insertion order; it serializes as a JS
    /// object (array-index keys first), the order JS signs it in
    #[serde(serialize_with = "serialize_js_object")]
    pub metas: OrderedMap<OrderedValue>,
}

impl TokenUnit {
    /// Create a new TokenUnit instance
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the token unit
    /// * `name` - Human-readable name
    /// * `metas` - Optional metadata map, in insertion order
    ///
    /// # Returns
    ///
    /// New TokenUnit instance
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    /// use knishio_client::OrderedMap;
    ///
    /// let token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     Some(OrderedMap::new())
    /// );
    /// ```
    pub fn new(id: String, name: String, metas: Option<OrderedMap<OrderedValue>>) -> Self {
        Self {
            id,
            name,
            metas: metas.unwrap_or_default(),
        }
    }

    /// Create a TokenUnit from GraphQL response data
    ///
    /// Equivalent to TokenUnit.createFromGraphQL() in JavaScript SDK. The ledger sends `metas`
    /// as JSON text; it is parsed in document order (JS `JSON.parse`).
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
        let id = data
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KnishIOError::custom("Missing 'id' field in GraphQL data"))?
            .to_string();

        let name = data
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KnishIOError::custom("Missing 'name' field in GraphQL data"))?
            .to_string();

        let metas = served_metas(data.get("metas"))?;

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
    /// use knishio_client::token_unit::TokenUnit;
    /// use knishio_client::OrderedValue;
    ///
    /// let data: OrderedValue = serde_json::from_str(r#"["token123", "My Token", {"fragmentZone": "zone1"}]"#).unwrap();
    /// let token_unit = TokenUnit::create_from_db(&data).unwrap();
    /// ```
    pub fn create_from_db(data: &OrderedValue) -> Result<Self> {
        let array = data
            .as_array()
            .ok_or_else(|| KnishIOError::custom("Expected array data for TokenUnit"))?;

        if array.len() < 2 {
            return Err(KnishIOError::custom(
                "Array must contain at least id and name",
            ));
        }

        let id = array[0]
            .as_str()
            .ok_or_else(|| KnishIOError::custom("First element (id) must be a string"))?
            .to_string();

        let name = array[1]
            .as_str()
            .ok_or_else(|| KnishIOError::custom("Second element (name) must be a string"))?
            .to_string();

        let metas = array
            .get(2)
            .and_then(OrderedValue::as_object)
            .cloned()
            .unwrap_or_default();

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
    ///
    /// let mut token_unit = TokenUnit::new("token123".to_string(), "My Token".to_string(), None);
    /// token_unit.set_meta("fragmentZone", "zone1");
    ///
    /// assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
    /// ```
    pub fn get_fragment_zone(&self) -> Option<String> {
        self.metas
            .get("fragmentZone")
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
    /// use serde_json::json;
    ///
    /// let mut token_unit = TokenUnit::new("token123".to_string(), "My Token".to_string(), None);
    /// token_unit.set_meta("fusedTokenUnits", json!(["unit1", "unit2"]));
    ///
    /// let fused_units = token_unit.get_fused_token_units().unwrap();
    /// assert_eq!(serde_json::Value::from(fused_units), json!(["unit1", "unit2"]));
    /// ```
    pub fn get_fused_token_units(&self) -> Option<OrderedValue> {
        self.metas.get("fusedTokenUnits").cloned()
    }

    /// Convert TokenUnit to array data format
    ///
    /// Equivalent to toData() in JavaScript SDK; the metas serialize in JS key order
    ///
    /// # Returns
    ///
    /// Array representation [id, name, metas]
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::token_unit::TokenUnit;
    ///
    /// let token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// let data = token_unit.to_data();
    /// assert_eq!(data[0].as_str(), Some("token123"));
    /// assert_eq!(data[1].as_str(), Some("My Token"));
    /// ```
    pub fn to_data(&self) -> Vec<OrderedValue> {
        vec![
            OrderedValue::String(self.id.clone()),
            OrderedValue::String(self.name.clone()),
            OrderedValue::Object(self.metas.clone()),
        ]
    }

    /// Convert TokenUnit to GraphQL response format
    ///
    /// Equivalent to toGraphQLResponse() in JavaScript SDK
    ///
    /// # Returns
    ///
    /// GraphQL-compatible response object (`metas` as JSON text in JS key order)
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
            "metas": OrderedValue::Object(self.metas.clone()).to_json_string()
        })
    }

    /// Set a metadata value (a JS property assignment: an existing key keeps its place)
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
    ///
    /// let mut token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// token_unit.set_meta("fragmentZone", "zone1");
    /// assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
    /// ```
    pub fn set_meta(&mut self, key: &str, value: impl Into<OrderedValue>) {
        self.metas.insert(key.to_string(), value.into());
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
    /// use knishio_client::OrderedValue;
    ///
    /// let mut token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// token_unit.set_meta("fragmentZone", "zone1");
    /// assert_eq!(token_unit.get_meta("fragmentZone"), Some(&OrderedValue::from("zone1")));
    /// ```
    pub fn get_meta(&self, key: &str) -> Option<&OrderedValue> {
        self.metas.get(key)
    }

    /// Remove a metadata value; the other keys keep their order
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
    /// use knishio_client::OrderedValue;
    ///
    /// let mut token_unit = TokenUnit::new(
    ///     "token123".to_string(),
    ///     "My Token".to_string(),
    ///     None
    /// );
    ///
    /// token_unit.set_meta("fragmentZone", "zone1");
    /// let removed = token_unit.remove_meta("fragmentZone");
    /// assert_eq!(removed, Some(OrderedValue::from("zone1")));
    /// assert_eq!(token_unit.get_fragment_zone(), None);
    /// ```
    pub fn remove_meta(&mut self, key: &str) -> Option<OrderedValue> {
        self.metas.shift_remove(key)
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
    /// Vector of all metadata keys, in insertion order
    pub fn metadata_keys(&self) -> Vec<String> {
        self.metas.keys().cloned().collect()
    }

    /// Clear all metadata
    pub fn clear_metadata(&mut self) {
        self.metas.clear();
    }
}

/// The `metas` of a unit served by the ledger, as JS `TokenUnit.createFromGraphQL` reads it.
///
/// JSON text is parsed in document order; text that is not JSON is an error (JS `JSON.parse`
/// throws). A served object is taken as is. Absent, null, empty text, or a parsed value that is
/// not an object reads as no metas.
pub(crate) fn served_metas(value: Option<&serde_json::Value>) -> Result<OrderedMap<OrderedValue>> {
    Ok(match value {
        Some(serde_json::Value::String(text)) if !text.is_empty() => {
            match serde_json::from_str::<OrderedValue>(text)? {
                OrderedValue::Object(map) => map,
                _ => OrderedMap::new(),
            }
        }
        Some(metas_value @ serde_json::Value::Object(_)) => {
            match OrderedValue::from(metas_value.clone()) {
                OrderedValue::Object(map) => map,
                _ => OrderedMap::new(),
            }
        }
        _ => OrderedMap::new(),
    })
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

    fn ov(value: serde_json::Value) -> OrderedValue {
        OrderedValue::from(value)
    }

    fn metas(entries: &[(&str, serde_json::Value)]) -> OrderedMap<OrderedValue> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), ov(v.clone())))
            .collect()
    }

    #[test]
    fn test_token_unit_new() {
        let token_unit = TokenUnit::new("token123".to_string(), "My Token".to_string(), None);

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert!(token_unit.metas.is_empty());
    }

    #[test]
    fn test_token_unit_new_with_metas() {
        let metas = metas(&[
            ("fragmentZone", json!("zone1")),
            ("fusedTokenUnits", json!(["unit1", "unit2"])),
        ]);

        let token_unit =
            TokenUnit::new("token123".to_string(), "My Token".to_string(), Some(metas));

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
        assert_eq!(
            token_unit.get_fused_token_units(),
            Some(ov(json!(["unit1", "unit2"])))
        );
    }

    /// The ledger's metas text keeps its document order; `to_data` writes it in JS key order.
    #[test]
    fn test_create_from_graphql_keeps_document_order(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let data = json!({ "id": "u1", "name": "U1", "metas": r#"{"zone":"Z","10":1,"2":2,"attr":{"b":1,"a":2}}"# });
        let token_unit = TokenUnit::create_from_graphql(&data)?;
        assert_eq!(token_unit.metadata_keys(), ["zone", "10", "2", "attr"]);
        assert_eq!(
            serde_json::to_string(&token_unit.to_data())?,
            r#"["u1","U1",{"2":2,"10":1,"zone":"Z","attr":{"b":1,"a":2}}]"#
        );
        assert_eq!(
            token_unit.to_graphql_response()["metas"],
            json!(r#"{"2":2,"10":1,"zone":"Z","attr":{"b":1,"a":2}}"#)
        );
        assert_eq!(
            serde_json::to_string(&token_unit)?,
            r#"{"id":"u1","name":"U1","metas":{"2":2,"10":1,"zone":"Z","attr":{"b":1,"a":2}}}"#
        );
        Ok(())
    }

    /// JS `JSON.parse` throws on metas that are not JSON; the call fails.
    #[test]
    fn test_create_from_graphql_rejects_metas_that_are_not_json() {
        for metas in ["{bad", "{}x", "nullx", "NaN"] {
            let data = json!({ "id": "u1", "name": "u1", "metas": metas });
            assert!(
                matches!(
                    TokenUnit::create_from_graphql(&data),
                    Err(KnishIOError::Serialization(_))
                ),
                "metas {metas:?}"
            );
        }
    }

    /// JS `JSON.parse(metas) || {}`: falsy served metas read as no metas.
    #[test]
    fn test_create_from_graphql_falsy_metas_are_empty(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        for metas in ["null", "false", "0", "\"\"", ""] {
            let data = json!({ "id": "u1", "name": "u1", "metas": metas });
            let unit = TokenUnit::create_from_graphql(&data)?;
            assert_eq!(
                serde_json::to_string(&unit.to_data())?,
                r#"["u1","u1",{}]"#,
                "metas {metas:?}"
            );
        }
        Ok(())
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
        assert_eq!(
            token_unit.get_fused_token_units(),
            Some(ov(json!(["unit1", "unit2"])))
        );
    }

    #[test]
    fn test_create_from_db() {
        let data = ov(json!([
            "token123",
            "My Token",
            {
                "fragmentZone": "zone1",
                "fusedTokenUnits": ["unit1", "unit2"]
            }
        ]));

        let token_unit = TokenUnit::create_from_db(&data).unwrap();

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));
        assert_eq!(
            token_unit.get_fused_token_units(),
            Some(ov(json!(["unit1", "unit2"])))
        );
    }

    #[test]
    fn test_create_from_db_minimal() {
        let data = ov(json!(["token123", "My Token"]));

        let token_unit = TokenUnit::create_from_db(&data).unwrap();

        assert_eq!(token_unit.id, "token123");
        assert_eq!(token_unit.name, "My Token");
        assert!(token_unit.metas.is_empty());
    }

    #[test]
    fn test_get_fragment_zone() {
        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas(&[("fragmentZone", json!("zone1"))])),
        );

        assert_eq!(token_unit.get_fragment_zone(), Some("zone1".to_string()));

        let token_unit_empty =
            TokenUnit::new("token456".to_string(), "Empty Token".to_string(), None);

        assert_eq!(token_unit_empty.get_fragment_zone(), None);
    }

    #[test]
    fn test_get_fused_token_units() {
        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas(&[("fusedTokenUnits", json!(["unit1", "unit2"]))])),
        );

        assert_eq!(
            token_unit.get_fused_token_units(),
            Some(ov(json!(["unit1", "unit2"])))
        );

        let token_unit_empty =
            TokenUnit::new("token456".to_string(), "Empty Token".to_string(), None);

        assert_eq!(token_unit_empty.get_fused_token_units(), None);
    }

    #[test]
    fn test_to_data() {
        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas(&[("fragmentZone", json!("zone1"))])),
        );

        let data = token_unit.to_data();

        assert_eq!(data.len(), 3);
        assert_eq!(data[0], ov(json!("token123")));
        assert_eq!(data[1], ov(json!("My Token")));

        // Check that metas is properly serialized
        assert_eq!(
            data[2]
                .as_object()
                .and_then(|metas| metas.get("fragmentZone")),
            Some(&ov(json!("zone1")))
        );
    }

    #[test]
    fn test_to_graphql_response() {
        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas(&[("fragmentZone", json!("zone1"))])),
        );

        let response = token_unit.to_graphql_response();

        assert_eq!(response["id"], json!("token123"));
        assert_eq!(response["name"], json!("My Token"));

        // Check that metas is properly JSON-encoded
        let metas_str = response["metas"].as_str().unwrap();
        assert_eq!(metas_str, r#"{"fragmentZone":"zone1"}"#);
    }

    #[test]
    fn test_metadata_operations() {
        let mut token_unit = TokenUnit::new("token123".to_string(), "My Token".to_string(), None);

        assert!(!token_unit.has_metadata());
        assert!(token_unit.metadata_keys().is_empty());

        // Set metadata
        token_unit.set_meta("key1", json!("value1"));
        token_unit.set_meta("key2", json!(42));

        assert!(token_unit.has_metadata());
        assert_eq!(token_unit.metadata_keys(), ["key1", "key2"]);

        // Get metadata
        assert_eq!(token_unit.get_meta("key1"), Some(&ov(json!("value1"))));
        assert_eq!(token_unit.get_meta("key2"), Some(&ov(json!(42))));
        assert_eq!(token_unit.get_meta("nonexistent"), None);

        // Remove metadata
        let removed = token_unit.remove_meta("key1");
        assert_eq!(removed, Some(ov(json!("value1"))));
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
        let invalid_array = ov(json!(["only_one_element"]));
        let result = TokenUnit::create_from_db(&invalid_array);
        assert!(result.is_err());

        // Test non-array data
        let invalid_data = ov(json!({"not": "array"}));
        let result = TokenUnit::create_from_db(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization() {
        let token_unit = TokenUnit::new(
            "token123".to_string(),
            "My Token".to_string(),
            Some(metas(&[("fragmentZone", json!("zone1"))])),
        );

        // Test JSON serialization
        let json_str = serde_json::to_string(&token_unit).unwrap();
        let deserialized: TokenUnit = serde_json::from_str(&json_str).unwrap();

        assert_eq!(token_unit, deserialized);
        assert_eq!(
            token_unit.get_fragment_zone(),
            deserialized.get_fragment_zone()
        );
    }

    #[test]
    fn test_display() {
        let token_unit = TokenUnit::new("token123".to_string(), "My Token".to_string(), None);

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
        let minimal_data = ov(json!(["token123", "My Token"]));
        let token_unit = TokenUnit::create_from_db(&minimal_data).unwrap();
        assert!(token_unit.metas.is_empty());

        let full_data = ov(json!(["token123", "My Token", {"key": "value"}]));
        let token_unit = TokenUnit::create_from_db(&full_data).unwrap();
        assert_eq!(token_unit.metas.len(), 1);
        assert_eq!(token_unit.get_meta("key"), Some(&ov(json!("value"))));

        // Test null handling for getFragmentZone (JavaScript line 103)
        let token_unit_no_zone = TokenUnit::new("test".to_string(), "test".to_string(), None);
        assert_eq!(token_unit_no_zone.get_fragment_zone(), None);

        // Test null handling for getFusedTokenUnits (JavaScript line 110)
        assert_eq!(token_unit_no_zone.get_fused_token_units(), None);
    }
}
