//! Condition implementation for Rules system
//!
//! Equivalent to Condition.js in the JavaScript SDK

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::Result;
use super::{RuleArgumentError, FromJsonObject, ToJsonObject};

/// Condition for rule evaluation
///
/// Equivalent to Condition class in JavaScript SDK
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Condition {
    /// The key to compare against
    pub key: String,
    /// The value to compare with
    pub value: String,
    /// The comparison operator (e.g., "==", "!=", ">", "<")
    pub comparison: String,
}

impl Condition {
    /// Create a new Condition
    ///
    /// Equivalent to Condition constructor in JavaScript
    ///
    /// # Arguments
    ///
    /// * `key` - The key to compare against
    /// * `value` - The value to compare with
    /// * `comparison` - The comparison operator
    ///
    /// # Returns
    ///
    /// New Condition instance
    ///
    /// # Errors
    ///
    /// Returns error if any parameter is empty
    pub fn new(key: String, value: String, comparison: String) -> Result<Self> {
        // Validate that all parameters are provided (equivalent to JS validation)
        if key.is_empty() || value.is_empty() || comparison.is_empty() {
            return Err(RuleArgumentError::new(
                "Condition::new() - not all class parameters are initialised!"
            ).into());
        }

        Ok(Self {
            key,
            value,
            comparison,
        })
    }

    /// Create a Condition from a JSON object
    ///
    /// Equivalent to Condition.toObject() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `object` - JSON object containing condition data
    ///
    /// # Returns
    ///
    /// New Condition instance
    pub fn from_object(object: &Value) -> Result<Self> {
        let key = object.get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuleArgumentError::new("Missing or invalid 'key' field"))?
            .to_string();

        let value = object.get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuleArgumentError::new("Missing or invalid 'value' field"))?
            .to_string();

        let comparison = object.get("comparison")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuleArgumentError::new("Missing or invalid 'comparison' field"))?
            .to_string();

        Self::new(key, value, comparison)
    }

    /// Convert to JSON object
    ///
    /// Equivalent to toJSON() in JavaScript
    ///
    /// # Returns
    ///
    /// JSON representation of the condition
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "key": self.key,
            "value": self.value,
            "comparison": self.comparison
        })
    }
}

impl FromJsonObject<Condition> for Condition {
    fn from_json_object(object: &Value) -> Result<Condition> {
        Self::from_object(object)
    }
}

impl ToJsonObject for Condition {
    fn to_json_object(&self) -> Value {
        self.to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_condition_new() {
        let condition = Condition::new(
            "balance".to_string(),
            "100".to_string(),
            ">=".to_string()
        ).unwrap();

        assert_eq!(condition.key, "balance");
        assert_eq!(condition.value, "100");
        assert_eq!(condition.comparison, ">=");
    }

    #[test]
    fn test_condition_new_empty_key() {
        let result = Condition::new(
            "".to_string(),
            "100".to_string(),
            ">=".to_string()
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not all class parameters are initialised"));
    }

    #[test]
    fn test_condition_new_empty_value() {
        let result = Condition::new(
            "balance".to_string(),
            "".to_string(),
            ">=".to_string()
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not all class parameters are initialised"));
    }

    #[test]
    fn test_condition_new_empty_comparison() {
        let result = Condition::new(
            "balance".to_string(),
            "100".to_string(),
            "".to_string()
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not all class parameters are initialised"));
    }

    #[test]
    fn test_condition_from_object() {
        let object = json!({
            "key": "token",
            "value": "TEST",
            "comparison": "=="
        });

        let condition = Condition::from_object(&object).unwrap();

        assert_eq!(condition.key, "token");
        assert_eq!(condition.value, "TEST");
        assert_eq!(condition.comparison, "==");
    }

    #[test]
    fn test_condition_from_object_missing_key() {
        let object = json!({
            "value": "TEST",
            "comparison": "=="
        });

        let result = Condition::from_object(&object);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing or invalid 'key' field"));
    }

    #[test]
    fn test_condition_from_object_missing_value() {
        let object = json!({
            "key": "token",
            "comparison": "=="
        });

        let result = Condition::from_object(&object);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing or invalid 'value' field"));
    }

    #[test]
    fn test_condition_from_object_missing_comparison() {
        let object = json!({
            "key": "token",
            "value": "TEST"
        });

        let result = Condition::from_object(&object);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing or invalid 'comparison' field"));
    }

    #[test]
    fn test_condition_to_json() {
        let condition = Condition::new(
            "amount".to_string(),
            "50".to_string(),
            ">".to_string()
        ).unwrap();

        let json_value = condition.to_json();

        assert_eq!(json_value["key"], "amount");
        assert_eq!(json_value["value"], "50");
        assert_eq!(json_value["comparison"], ">");
    }

    #[test]
    fn test_condition_serialization() {
        let condition = Condition::new(
            "status".to_string(),
            "active".to_string(),
            "==".to_string()
        ).unwrap();

        // Test JSON serialization
        let json_str = serde_json::to_string(&condition).unwrap();
        let deserialized: Condition = serde_json::from_str(&json_str).unwrap();

        assert_eq!(condition, deserialized);
    }

    #[test]
    fn test_condition_traits() {
        let object = json!({
            "key": "balance",
            "value": "1000",
            "comparison": "<="
        });

        // Test FromJsonObject trait
        let condition = Condition::from_json_object(&object).unwrap();
        assert_eq!(condition.key, "balance");
        assert_eq!(condition.value, "1000");
        assert_eq!(condition.comparison, "<=");

        // Test ToJsonObject trait
        let json_output = condition.to_json_object();
        assert_eq!(json_output["key"], "balance");
        assert_eq!(json_output["value"], "1000");
        assert_eq!(json_output["comparison"], "<=");
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test that our implementation matches JavaScript SDK patterns

        // Test constructor validation (matches JS behavior)
        let valid_condition = Condition::new(
            "test_key".to_string(),
            "test_value".to_string(),
            "test_comparison".to_string()
        );
        assert!(valid_condition.is_ok());

        // Test empty parameter validation (matches JS validation)
        let invalid_conditions = vec![
            Condition::new("".to_string(), "value".to_string(), "comp".to_string()),
            Condition::new("key".to_string(), "".to_string(), "comp".to_string()),
            Condition::new("key".to_string(), "value".to_string(), "".to_string()),
        ];

        for invalid in invalid_conditions {
            assert!(invalid.is_err());
        }

        // Test JSON object creation (matches JS toObject method)
        let js_style_object = json!({
            "key": "wallet_balance",
            "value": "500",
            "comparison": ">="
        });

        let condition = Condition::from_object(&js_style_object).unwrap();
        let json_output = condition.to_json();

        // Verify round-trip compatibility
        assert_eq!(json_output["key"], js_style_object["key"]);
        assert_eq!(json_output["value"], js_style_object["value"]);
        assert_eq!(json_output["comparison"], js_style_object["comparison"]);
    }
}