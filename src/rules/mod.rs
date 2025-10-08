//! Rules infrastructure for KnishIO DLT
//!
//! This module provides the Rules system matching the JavaScript SDK functionality.
//! Rules consist of Conditions and Callbacks that define validation and action patterns.

use serde_json::Value;
use crate::error::{KnishIOError, Result};

pub mod rule;
pub mod callback;
pub mod condition;

pub use rule::Rule;
pub use callback::Callback;
pub use condition::Condition;

/// Exception for rule argument validation errors
///
/// Equivalent to RuleArgumentException in JavaScript
#[derive(thiserror::Error, Debug)]
#[error("Rule argument error: {message}")]
pub struct RuleArgumentError {
    pub message: String,
}

impl RuleArgumentError {
    /// Create a new rule argument error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<RuleArgumentError> for KnishIOError {
    fn from(err: RuleArgumentError) -> Self {
        KnishIOError::custom(err.message)
    }
}

/// Trait for objects that can be converted from JSON
pub trait FromJsonObject<T> {
    /// Convert from a JSON object to the target type
    fn from_json_object(object: &Value) -> Result<T>;
}

/// Trait for objects that can be converted to JSON
pub trait ToJsonObject {
    /// Convert to a JSON value
    fn to_json_object(&self) -> Value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rule_argument_error() {
        let error = RuleArgumentError::new("Test error message");
        assert_eq!(error.message, "Test error message");
        assert_eq!(error.to_string(), "Rule argument error: Test error message");
    }

    #[test]
    fn test_rule_argument_error_conversion() {
        let rule_error = RuleArgumentError::new("Test conversion");
        let knish_error: KnishIOError = rule_error.into();
        assert!(knish_error.to_string().contains("Test conversion"));
    }

    #[test]
    fn test_cross_sdk_rule_json_compatibility() {
        // Test that our Rule JSON output matches JavaScript SDK exactly
        use crate::rules::{Rule, Condition, Callback};
        use callback::Meta;

        // Create a complex rule matching JavaScript SDK test patterns
        let condition = Condition::new(
            "balance".to_string(),
            "1000".to_string(),
            ">=".to_string()
        ).unwrap();

        let mut meta = Meta::empty();
        meta.set_property("type".to_string(), json!("validation"));
        meta.set_property("level".to_string(), json!("strict"));

        let callback = Callback::new(
            "collect".to_string(),
            Some("policy".to_string()),
            Some("balance_check".to_string()),
            Some(meta),
            Some("target_address".to_string()),
            Some("COLLECT".to_string()),
            Some("250".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        let rule = Rule::new(vec![condition], vec![callback]);
        let json_output = rule.to_json();

        // Verify JSON structure matches JavaScript SDK format
        assert!(json_output.get("condition").is_some());
        assert!(json_output.get("callback").is_some());
        
        let conditions = json_output["condition"].as_array().unwrap();
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0]["key"], "balance");
        assert_eq!(conditions[0]["value"], "1000");
        assert_eq!(conditions[0]["comparison"], ">=");

        let callbacks = json_output["callback"].as_array().unwrap();
        assert_eq!(callbacks.len(), 1);
        assert_eq!(callbacks[0]["action"], "collect");
        assert_eq!(callbacks[0]["metaType"], "policy");
        assert_eq!(callbacks[0]["metaId"], "balance_check");
        assert_eq!(callbacks[0]["address"], "target_address");
        assert_eq!(callbacks[0]["token"], "COLLECT");
        assert_eq!(callbacks[0]["amount"], "250");
        assert_eq!(callbacks[0]["comparison"], ">=");

        // Verify Meta object structure
        let meta_obj = &callbacks[0]["meta"];
        assert_eq!(meta_obj["type"], "validation");
        assert_eq!(meta_obj["level"], "strict");
    }

    #[test]
    fn test_cross_sdk_callback_action_validation() {
        // Test that action validation matches JavaScript SDK exactly
        use crate::rules::Callback;
        use callback::Meta;

        // Test reject callback (minimal requirements)
        let reject_callback = Callback::new(
            "reject".to_string(),
            None, None, None, None, None, None, None,
        ).unwrap();
        assert!(reject_callback.is_reject());
        assert!(!reject_callback.is_collect());
        assert!(!reject_callback.is_meta());

        // Test meta callback (requires action, metaId, metaType, meta)
        let mut meta = Meta::empty();
        meta.set_property("test".to_string(), json!("value"));
        
        let meta_callback = Callback::new(
            "meta".to_string(),
            Some("test_type".to_string()),
            Some("test_id".to_string()),
            Some(meta),
            None, None, None, None,
        ).unwrap();
        assert!(meta_callback.is_meta());
        assert!(!meta_callback.is_collect());
        assert!(!meta_callback.is_reject());

        // Test collect callback (requires action, address, token, amount, comparison)
        let collect_callback = Callback::new(
            "collect".to_string(),
            None, None, None,
            Some("test_address".to_string()),
            Some("TEST".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();
        assert!(collect_callback.is_collect());
        assert!(!collect_callback.is_meta());
        assert!(!collect_callback.is_reject());

        // Test buffer callback (same requirements as collect)
        let buffer_callback = Callback::new(
            "buffer".to_string(),
            None, None, None,
            Some("buffer_address".to_string()),
            Some("BUFFER".to_string()),
            Some("200".to_string()),
            Some(">".to_string()),
        ).unwrap();
        assert!(buffer_callback.is_buffer());
        assert!(!buffer_callback.is_collect());

        // Test remit callback (requires action, token, amount)
        let remit_callback = Callback::new(
            "remit".to_string(),
            None, None, None, None,
            Some("REMIT".to_string()),
            Some("150".to_string()),
            None,
        ).unwrap();
        assert!(remit_callback.is_remit());
        assert!(!remit_callback.is_collect());

        // Test burn callback (requires action, token, amount, comparison)
        let burn_callback = Callback::new(
            "burn".to_string(),
            None, None, None, None,
            Some("BURN".to_string()),
            Some("50".to_string()),
            Some("<".to_string()),
        ).unwrap();
        assert!(burn_callback.is_burn());
        assert!(!burn_callback.is_remit());
    }

    #[test]
    fn test_cross_sdk_rule_validation_errors() {
        // Test that error handling matches JavaScript SDK exactly
        use crate::rules::Rule;

        // Test missing condition field (should match JS error message)
        let invalid_rule_no_condition = json!({
            "callback": [
                {"action": "reject"}
            ]
        });

        let result = Rule::from_object(&invalid_rule_no_condition);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("no condition field"));

        // Test missing callback field (should match JS error message)
        let invalid_rule_no_callback = json!({
            "condition": [
                {"key": "test", "value": "value", "comparison": "=="}
            ]
        });

        let result = Rule::from_object(&invalid_rule_no_callback);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("no callback field"));

        // Test invalid condition format
        let invalid_rule_bad_condition = json!({
            "condition": "not_an_array",
            "callback": []
        });

        let result = Rule::from_object(&invalid_rule_bad_condition);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("condition field must be an array"));

        // Test invalid callback format
        let invalid_rule_bad_callback = json!({
            "condition": [],
            "callback": "not_an_array"
        });

        let result = Rule::from_object(&invalid_rule_bad_callback);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("callback field must be an array"));
    }

    #[test]
    fn test_cross_sdk_numeric_validation() {
        // Test that numeric validation matches JavaScript SDK exactly
        use crate::rules::Callback;

        // Valid numeric amounts should work
        let valid_amounts = vec!["0", "100", "1000.50", "0.001", "999999"];
        for amount in valid_amounts {
            let result = Callback::new(
                "collect".to_string(),
                None, None, None,
                Some("address".to_string()),
                Some("TOKEN".to_string()),
                Some(amount.to_string()),
                Some(">=".to_string()),
            );
            assert!(result.is_ok(), "Amount '{}' should be valid", amount);
        }

        // Invalid amounts should fail
        let invalid_amounts = vec!["", "abc", "10.20.30", "not_a_number", "100a"];
        for amount in invalid_amounts {
            let result = Callback::new(
                "collect".to_string(),
                None, None, None,
                Some("address".to_string()),
                Some("TOKEN".to_string()),
                Some(amount.to_string()),
                Some(">=".to_string()),
            );
            assert!(result.is_err(), "Amount '{}' should be invalid", amount);
            let error = result.unwrap_err().to_string();
            assert!(error.contains("should be a string containing numbers"));
        }
    }

    #[test]
    fn test_cross_sdk_round_trip_compatibility() {
        // Test that JSON round-trip maintains compatibility with JavaScript SDK
        use crate::rules::{Rule, Condition, Callback};
        use callback::Meta;

        // Create a complex rule structure
        let condition1 = Condition::new(
            "wallet_balance".to_string(),
            "500".to_string(),
            ">=".to_string()
        ).unwrap();

        let condition2 = Condition::new(
            "token_type".to_string(),
            "PREMIUM".to_string(),
            "==".to_string()
        ).unwrap();

        let mut meta = Meta::empty();
        meta.set_property("validation_level".to_string(), json!("high"));
        meta.set_property("priority".to_string(), json!(1));
        meta.set_property("enabled".to_string(), json!(true));

        let callback1 = Callback::new(
            "collect".to_string(),
            Some("reward".to_string()),
            Some("premium_bonus".to_string()),
            Some(meta),
            Some("reward_pool_address".to_string()),
            Some("REWARD".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        let callback2 = Callback::new(
            "reject".to_string(),
            None, None, None, None, None, None, None,
        ).unwrap();

        let original_rule = Rule::new(vec![condition1, condition2], vec![callback1, callback2]);
        
        // Convert to JSON and back
        let json_output = original_rule.to_json();
        let restored_rule = Rule::from_object(&json_output).unwrap();

        // Verify complete compatibility
        assert_eq!(original_rule.condition_count(), restored_rule.condition_count());
        assert_eq!(original_rule.callback_count(), restored_rule.callback_count());

        // Verify conditions are preserved
        let original_conditions = original_rule.get_conditions();
        let restored_conditions = restored_rule.get_conditions();
        for (orig, rest) in original_conditions.iter().zip(restored_conditions.iter()) {
            assert_eq!(orig.key, rest.key);
            assert_eq!(orig.value, rest.value);
            assert_eq!(orig.comparison, rest.comparison);
        }

        // Verify callbacks are preserved
        let original_callbacks = original_rule.get_callbacks();
        let restored_callbacks = restored_rule.get_callbacks();
        for (orig, rest) in original_callbacks.iter().zip(restored_callbacks.iter()) {
            assert_eq!(orig.action, rest.action);
            assert_eq!(orig.meta_type, rest.meta_type);
            assert_eq!(orig.meta_id, rest.meta_id);
            assert_eq!(orig.address, rest.address);
            assert_eq!(orig.token, rest.token);
            assert_eq!(orig.amount, rest.amount);
            assert_eq!(orig.comparison, rest.comparison);
        }

        // Verify action type validation still works
        assert!(restored_callbacks[0].is_collect());
        assert!(restored_callbacks[1].is_reject());
    }

    #[test]
    fn test_cross_sdk_meta_dynamic_properties() {
        // Test that Meta dynamic properties work exactly like JavaScript SDK
        use callback::Meta;

        let mut meta = Meta::empty();
        
        // Test dynamic property assignment (like JavaScript)
        meta.set_property("stringProp".to_string(), json!("test_value"));
        meta.set_property("numberProp".to_string(), json!(42));
        meta.set_property("booleanProp".to_string(), json!(true));
        meta.set_property("arrayProp".to_string(), json!(["item1", "item2"]));
        meta.set_property("objectProp".to_string(), json!({"nested": "value"}));

        // Test property access
        assert_eq!(meta.get_property("stringProp"), Some(&json!("test_value")));
        assert_eq!(meta.get_property("numberProp"), Some(&json!(42)));
        assert_eq!(meta.get_property("booleanProp"), Some(&json!(true)));
        assert_eq!(meta.get_property("arrayProp"), Some(&json!(["item1", "item2"])));
        assert_eq!(meta.get_property("objectProp"), Some(&json!({"nested": "value"})));

        // Test property existence
        assert!(meta.has_property("stringProp"));
        assert!(meta.has_property("numberProp"));
        assert!(!meta.has_property("nonexistent"));

        // Test keys collection
        let keys = meta.keys();
        assert_eq!(keys.len(), 5);
        assert!(keys.contains(&"stringProp".to_string()));
        assert!(keys.contains(&"numberProp".to_string()));
        assert!(keys.contains(&"booleanProp".to_string()));
        assert!(keys.contains(&"arrayProp".to_string()));
        assert!(keys.contains(&"objectProp".to_string()));

        // Test JSON serialization
        let json_output = meta.to_json();
        assert_eq!(json_output["stringProp"], "test_value");
        assert_eq!(json_output["numberProp"], 42);
        assert_eq!(json_output["booleanProp"], true);
        assert!(json_output["arrayProp"].is_array());
        assert!(json_output["objectProp"].is_object());

        // Test round-trip through JSON
        let restored_meta = Meta::from_object(&json_output).unwrap();
        assert_eq!(restored_meta.get_property("stringProp"), Some(&json!("test_value")));
        assert_eq!(restored_meta.get_property("numberProp"), Some(&json!(42)));
        assert_eq!(restored_meta.get_property("booleanProp"), Some(&json!(true)));
    }
}