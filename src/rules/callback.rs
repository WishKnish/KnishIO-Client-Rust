//! Callback implementation for Rules system
//!
//! Equivalent to Callback.js in the JavaScript SDK

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use crate::error::Result;
use crate::utils::strings::is_numeric;
use super::{RuleArgumentError, FromJsonObject, ToJsonObject};

/// Metadata for callbacks
///
/// Full representation of Meta class from JavaScript SDK supporting dynamic properties
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Meta {
    /// Dynamic properties map supporting arbitrary key-value pairs
    #[serde(flatten)]
    pub properties: HashMap<String, serde_json::Value>,
}

impl Meta {
    /// Create a new Meta with a single key-value pair
    ///
    /// Equivalent to JavaScript Meta with dynamic property assignment
    pub fn new(key: String, value: String) -> Self {
        let mut properties = HashMap::new();
        properties.insert(key, serde_json::Value::String(value));
        Self { properties }
    }

    /// Create a new empty Meta
    pub fn empty() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    /// Create Meta from a JSON object
    ///
    /// Equivalent to JavaScript Meta creation with dynamic properties
    pub fn from_object(object: &Value) -> Result<Self> {
        let mut properties = HashMap::new();
        
        if let Some(obj) = object.as_object() {
            for (key, value) in obj {
                properties.insert(key.clone(), value.clone());
            }
        }
        
        Ok(Self { properties })
    }

    /// Set a property value
    ///
    /// Equivalent to JavaScript dynamic property assignment
    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
    }

    /// Get a property value
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    /// Remove a property
    pub fn remove_property(&mut self, key: &str) -> Option<serde_json::Value> {
        self.properties.remove(key)
    }

    /// Check if a property exists
    pub fn has_property(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Get all property keys
    pub fn keys(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// Check if Meta is empty
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Clear all properties
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// Convert to JSON Value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.properties).unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
    }

    /// Legacy getter for backward compatibility
    /// 
    /// Returns the value of the first property if any exists
    pub fn key(&self) -> Option<String> {
        self.properties.keys().next().cloned()
    }

    /// Legacy getter for backward compatibility
    ///
    /// Returns the value of the first property if any exists
    pub fn value(&self) -> Option<String> {
        self.properties.values().next()
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

/// Callback for rule actions
///
/// Equivalent to Callback class in JavaScript SDK
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Callback {
    /// The action to be performed (required)
    pub action: String,
    /// Optional metadata type
    pub meta_type: Option<String>,
    /// Optional metadata ID
    pub meta_id: Option<String>,
    /// Optional metadata object
    pub meta: Option<Meta>,
    /// Optional address
    pub address: Option<String>,
    /// Optional token
    pub token: Option<String>,
    /// Optional amount (must be numeric string)
    pub amount: Option<String>,
    /// Optional comparison operator
    pub comparison: Option<String>,
}

impl Callback {
    /// Create a new Callback
    ///
    /// Equivalent to Callback constructor in JavaScript
    ///
    /// # Arguments
    ///
    /// * `action` - The action to be performed (required)
    /// * `meta_type` - Optional metadata type
    /// * `meta_id` - Optional metadata ID
    /// * `meta` - Optional metadata object
    /// * `address` - Optional address
    /// * `token` - Optional token
    /// * `amount` - Optional amount (must be numeric string)
    /// * `comparison` - Optional comparison operator
    ///
    /// # Returns
    ///
    /// New Callback instance
    ///
    /// # Errors
    ///
    /// Returns error if action is not provided
    pub fn new(
        action: String,
        meta_type: Option<String>,
        meta_id: Option<String>,
        meta: Option<Meta>,
        address: Option<String>,
        token: Option<String>,
        amount: Option<String>,
        comparison: Option<String>,
    ) -> Result<Self> {
        if action.is_empty() {
            return Err(RuleArgumentError::new(
                "Callback structure violated, missing mandatory \"action\" parameter."
            ).into());
        }

        // Validate amount if provided (must be numeric)
        if let Some(ref amount_val) = amount {
            if !is_numeric(amount_val) {
                return Err(RuleArgumentError::new(
                    "Parameter amount should be a string containing numbers"
                ).into());
            }
        }

        Ok(Self {
            action,
            meta_type,
            meta_id,
            meta,
            address,
            token,
            amount,
            comparison,
        })
    }

    /// Set the comparison operator
    ///
    /// Equivalent to comparison setter in JavaScript
    pub fn set_comparison(&mut self, comparison: String) {
        self.comparison = Some(comparison);
    }

    /// Set the amount
    ///
    /// Equivalent to amount setter in JavaScript
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount as numeric string
    ///
    /// # Errors
    ///
    /// Returns error if amount is not numeric
    pub fn set_amount(&mut self, amount: String) -> Result<()> {
        if !is_numeric(&amount) {
            return Err(RuleArgumentError::new(
                "Parameter amount should be a string containing numbers"
            ).into());
        }
        self.amount = Some(amount);
        Ok(())
    }

    /// Set the token
    ///
    /// Equivalent to token setter in JavaScript
    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    /// Set the address
    ///
    /// Equivalent to address setter in JavaScript
    pub fn set_address(&mut self, address: String) {
        self.address = Some(address);
    }

    /// Set the metadata
    ///
    /// Equivalent to meta setter in JavaScript
    pub fn set_meta(&mut self, meta: Meta) {
        self.meta = Some(meta);
    }

    /// Set the metadata type
    ///
    /// Equivalent to metaType setter in JavaScript
    pub fn set_meta_type(&mut self, meta_type: String) {
        self.meta_type = Some(meta_type);
    }

    /// Set the metadata ID
    ///
    /// Equivalent to metaId setter in JavaScript
    pub fn set_meta_id(&mut self, meta_id: String) {
        self.meta_id = Some(meta_id);
    }

    /// Create a Callback from a JSON object
    ///
    /// Equivalent to Callback.toObject() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `object` - JSON object containing callback data
    ///
    /// # Returns
    ///
    /// New Callback instance
    pub fn from_object(object: &Value) -> Result<Self> {
        let action = object.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuleArgumentError::new("Missing or invalid 'action' field"))?
            .to_string();

        let meta_type = object.get("metaType")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let meta_id = object.get("metaId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let meta = if let Some(meta_obj) = object.get("meta") {
            Some(Meta::from_object(meta_obj)?)
        } else {
            None
        };

        let address = object.get("address")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let token = object.get("token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let amount = object.get("amount")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let comparison = object.get("comparison")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Self::new(action, meta_type, meta_id, meta, address, token, amount, comparison)
    }

    /// Convert to JSON object
    ///
    /// Equivalent to toJSON() in JavaScript
    ///
    /// # Returns
    ///
    /// JSON representation of the callback
    pub fn to_json(&self) -> Value {
        let mut meta = serde_json::Map::new();
        meta.insert("action".to_string(), Value::String(self.action.clone()));

        if let Some(ref meta_type) = self.meta_type {
            meta.insert("metaType".to_string(), Value::String(meta_type.clone()));
        }

        if let Some(ref meta_id) = self.meta_id {
            meta.insert("metaId".to_string(), Value::String(meta_id.clone()));
        }

        if let Some(ref meta_obj) = self.meta {
            meta.insert("meta".to_string(), serde_json::to_value(meta_obj).unwrap_or(Value::Null));
        }

        if let Some(ref address) = self.address {
            meta.insert("address".to_string(), Value::String(address.clone()));
        }

        if let Some(ref token) = self.token {
            meta.insert("token".to_string(), Value::String(token.clone()));
        }

        if let Some(ref amount) = self.amount {
            meta.insert("amount".to_string(), Value::String(amount.clone()));
        }

        if let Some(ref comparison) = self.comparison {
            meta.insert("comparison".to_string(), Value::String(comparison.clone()));
        }

        Value::Object(meta)
    }

    /// Check if this is a reject callback
    ///
    /// Equivalent to isReject() in JavaScript
    pub fn is_reject(&self) -> bool {
        self.is_action("reject")
    }

    /// Check if this is a meta callback
    ///
    /// Equivalent to isMeta() in JavaScript
    pub fn is_meta(&self) -> bool {
        let json = self.to_json();
        let keys: HashSet<String> = json.as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let required_keys: HashSet<String> = ["action", "metaId", "metaType", "meta"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let intersection: HashSet<_> = keys.intersection(&required_keys).collect();
        intersection.len() == 4 && self.is_action("meta")
    }

    /// Check if this is a collect callback
    ///
    /// Equivalent to isCollect() in JavaScript
    pub fn is_collect(&self) -> bool {
        let json = self.to_json();
        let keys: HashSet<String> = json.as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let required_keys: HashSet<String> = ["action", "address", "token", "amount", "comparison"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let intersection: HashSet<_> = keys.intersection(&required_keys).collect();
        intersection.len() == 5 && self.is_action("collect")
    }

    /// Check if this is a buffer callback
    ///
    /// Equivalent to isBuffer() in JavaScript
    pub fn is_buffer(&self) -> bool {
        let json = self.to_json();
        let keys: HashSet<String> = json.as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let required_keys: HashSet<String> = ["action", "address", "token", "amount", "comparison"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let intersection: HashSet<_> = keys.intersection(&required_keys).collect();
        intersection.len() == 5 && self.is_action("buffer")
    }

    /// Check if this is a remit callback
    ///
    /// Equivalent to isRemit() in JavaScript
    pub fn is_remit(&self) -> bool {
        let json = self.to_json();
        let keys: HashSet<String> = json.as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let required_keys: HashSet<String> = ["action", "token", "amount"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let intersection: HashSet<_> = keys.intersection(&required_keys).collect();
        intersection.len() == 3 && self.is_action("remit")
    }

    /// Check if this is a burn callback
    ///
    /// Equivalent to isBurn() in JavaScript
    pub fn is_burn(&self) -> bool {
        let json = self.to_json();
        let keys: HashSet<String> = json.as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let required_keys: HashSet<String> = ["action", "token", "amount", "comparison"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let intersection: HashSet<_> = keys.intersection(&required_keys).collect();
        intersection.len() == 4 && self.is_action("burn")
    }

    /// Private helper method to check action type
    ///
    /// Equivalent to _is() in JavaScript
    fn is_action(&self, action_type: &str) -> bool {
        self.action.to_lowercase() == action_type.to_lowercase()
    }
}

impl FromJsonObject<Callback> for Callback {
    fn from_json_object(object: &Value) -> Result<Callback> {
        Self::from_object(object)
    }
}

impl ToJsonObject for Callback {
    fn to_json_object(&self) -> Value {
        self.to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_meta_creation() {
        let meta = Meta::new("test_key".to_string(), "test_value".to_string());
        assert_eq!(meta.key(), Some("test_key".to_string()));
        assert_eq!(meta.value(), Some("test_value".to_string()));
    }

    #[test]
    fn test_meta_from_object() {
        let object = json!({
            "key": "meta_key",
            "value": "meta_value"
        });

        let meta = Meta::from_object(&object).unwrap();
        assert_eq!(meta.key(), Some("meta_key".to_string()));
        assert_eq!(meta.value(), Some("meta_value".to_string()));
    }

    #[test]
    fn test_callback_new() {
        let callback = Callback::new(
            "test_action".to_string(),
            Some("test_meta_type".to_string()),
            Some("test_meta_id".to_string()),
            None,
            Some("test_address".to_string()),
            Some("TEST".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        assert_eq!(callback.action, "test_action");
        assert_eq!(callback.meta_type, Some("test_meta_type".to_string()));
        assert_eq!(callback.meta_id, Some("test_meta_id".to_string()));
        assert_eq!(callback.address, Some("test_address".to_string()));
        assert_eq!(callback.token, Some("TEST".to_string()));
        assert_eq!(callback.amount, Some("100".to_string()));
        assert_eq!(callback.comparison, Some(">=".to_string()));
    }

    #[test]
    fn test_callback_new_empty_action() {
        let result = Callback::new(
            "".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing mandatory \"action\" parameter"));
    }

    #[test]
    fn test_callback_new_invalid_amount() {
        let result = Callback::new(
            "test_action".to_string(),
            None,
            None,
            None,
            None,
            None,
            Some("not_a_number".to_string()),
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("string containing numbers"));
    }

    #[test]
    fn test_callback_setters() {
        let mut callback = Callback::new(
            "test_action".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();

        callback.set_comparison(">=".to_string());
        assert_eq!(callback.comparison, Some(">=".to_string()));

        callback.set_amount("500".to_string()).unwrap();
        assert_eq!(callback.amount, Some("500".to_string()));

        callback.set_token("TOKEN".to_string());
        assert_eq!(callback.token, Some("TOKEN".to_string()));

        callback.set_address("addr123".to_string());
        assert_eq!(callback.address, Some("addr123".to_string()));

        let meta = Meta::new("key".to_string(), "value".to_string());
        callback.set_meta(meta.clone());
        assert_eq!(callback.meta, Some(meta));

        callback.set_meta_type("type1".to_string());
        assert_eq!(callback.meta_type, Some("type1".to_string()));

        callback.set_meta_id("id1".to_string());
        assert_eq!(callback.meta_id, Some("id1".to_string()));
    }

    #[test]
    fn test_callback_set_invalid_amount() {
        let mut callback = Callback::new(
            "test_action".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();

        let result = callback.set_amount("invalid_amount".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("string containing numbers"));
    }

    #[test]
    fn test_callback_from_object() {
        let object = json!({
            "action": "collect",
            "metaType": "test_type",
            "metaId": "test_id",
            "address": "test_address",
            "token": "TEST",
            "amount": "100",
            "comparison": ">="
        });

        let callback = Callback::from_object(&object).unwrap();

        assert_eq!(callback.action, "collect");
        assert_eq!(callback.meta_type, Some("test_type".to_string()));
        assert_eq!(callback.meta_id, Some("test_id".to_string()));
        assert_eq!(callback.address, Some("test_address".to_string()));
        assert_eq!(callback.token, Some("TEST".to_string()));
        assert_eq!(callback.amount, Some("100".to_string()));
        assert_eq!(callback.comparison, Some(">=".to_string()));
    }

    #[test]
    fn test_callback_from_object_with_meta() {
        let object = json!({
            "action": "meta",
            "metaType": "test_type",
            "metaId": "test_id",
            "meta": {
                "key": "test_key",
                "value": "test_value"
            }
        });

        let callback = Callback::from_object(&object).unwrap();

        assert_eq!(callback.action, "meta");
        assert_eq!(callback.meta_type, Some("test_type".to_string()));
        assert_eq!(callback.meta_id, Some("test_id".to_string()));
        assert!(callback.meta.is_some());

        let meta = callback.meta.unwrap();
        assert_eq!(meta.key(), Some("test_key".to_string()));
        assert_eq!(meta.value(), Some("test_value".to_string()));
    }

    #[test]
    fn test_callback_to_json() {
        let callback = Callback::new(
            "collect".to_string(),
            Some("test_type".to_string()),
            Some("test_id".to_string()),
            None,
            Some("test_address".to_string()),
            Some("TEST".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        let json_value = callback.to_json();

        assert_eq!(json_value["action"], "collect");
        assert_eq!(json_value["metaType"], "test_type");
        assert_eq!(json_value["metaId"], "test_id");
        assert_eq!(json_value["address"], "test_address");
        assert_eq!(json_value["token"], "TEST");
        assert_eq!(json_value["amount"], "100");
        assert_eq!(json_value["comparison"], ">=");
    }

    #[test]
    fn test_callback_is_reject() {
        let callback = Callback::new("reject".to_string(), None, None, None, None, None, None, None).unwrap();
        assert!(callback.is_reject());

        let callback2 = Callback::new("REJECT".to_string(), None, None, None, None, None, None, None).unwrap();
        assert!(callback2.is_reject());

        let callback3 = Callback::new("collect".to_string(), None, None, None, None, None, None, None).unwrap();
        assert!(!callback3.is_reject());
    }

    #[test]
    fn test_callback_is_meta() {
        let meta = Meta::new("key".to_string(), "value".to_string());
        let callback = Callback::new(
            "meta".to_string(),
            Some("type".to_string()),
            Some("id".to_string()),
            Some(meta),
            None,
            None,
            None,
            None,
        ).unwrap();

        assert!(callback.is_meta());

        // Missing required fields
        let callback2 = Callback::new(
            "meta".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();
        assert!(!callback2.is_meta());
    }

    #[test]
    fn test_callback_is_collect() {
        let callback = Callback::new(
            "collect".to_string(),
            None,
            None,
            None,
            Some("address".to_string()),
            Some("TOKEN".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        assert!(callback.is_collect());

        // Missing required fields
        let callback2 = Callback::new(
            "collect".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();
        assert!(!callback2.is_collect());
    }

    #[test]
    fn test_callback_is_buffer() {
        let callback = Callback::new(
            "buffer".to_string(),
            None,
            None,
            None,
            Some("address".to_string()),
            Some("TOKEN".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        assert!(callback.is_buffer());
    }

    #[test]
    fn test_callback_is_remit() {
        let callback = Callback::new(
            "remit".to_string(),
            None,
            None,
            None,
            None,
            Some("TOKEN".to_string()),
            Some("100".to_string()),
            None,
        ).unwrap();

        assert!(callback.is_remit());
    }

    #[test]
    fn test_callback_is_burn() {
        let callback = Callback::new(
            "burn".to_string(),
            None,
            None,
            None,
            None,
            Some("TOKEN".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        assert!(callback.is_burn());
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test that our implementation matches JavaScript SDK patterns

        // Test constructor validation (matches JS behavior)
        let valid_callback = Callback::new(
            "test_action".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(valid_callback.is_ok());

        // Test empty action validation (matches JS validation)
        let invalid_callback = Callback::new(
            "".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(invalid_callback.is_err());

        // Test amount validation (matches JS validation)
        let invalid_amount = Callback::new(
            "action".to_string(),
            None,
            None,
            None,
            None,
            None,
            Some("not_numeric".to_string()),
            None,
        );
        assert!(invalid_amount.is_err());

        // Test JSON object creation (matches JS toObject method)
        let js_style_object = json!({
            "action": "collect",
            "address": "test_address",
            "token": "TEST",
            "amount": "500",
            "comparison": ">"
        });

        let callback = Callback::from_object(&js_style_object).unwrap();
        let json_output = callback.to_json();

        // Verify round-trip compatibility
        assert_eq!(json_output["action"], js_style_object["action"]);
        assert_eq!(json_output["address"], js_style_object["address"]);
        assert_eq!(json_output["token"], js_style_object["token"]);
        assert_eq!(json_output["amount"], js_style_object["amount"]);
        assert_eq!(json_output["comparison"], js_style_object["comparison"]);

        // Test action type checking (matches JS _is method)
        assert!(callback.is_collect());
        assert!(!callback.is_reject());
        assert!(!callback.is_burn());
    }
}