//! Rule implementation for Rules system
//!
//! Equivalent to Rule.js in the JavaScript SDK

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::Result;
use super::{Condition, Callback, RuleArgumentError, FromJsonObject, ToJsonObject};

/// Rule containing conditions and callbacks
///
/// Equivalent to Rule class in JavaScript SDK
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    /// Collection of conditions for this rule
    pub conditions: Vec<Condition>,
    /// Collection of callbacks for this rule
    pub callbacks: Vec<Callback>,
}

impl Rule {
    /// Create a new Rule
    ///
    /// Equivalent to Rule constructor in JavaScript
    ///
    /// # Arguments
    ///
    /// * `conditions` - Vector of conditions
    /// * `callbacks` - Vector of callbacks
    ///
    /// # Returns
    ///
    /// New Rule instance
    ///
    /// # Note
    ///
    /// In Rust, type safety ensures that only Condition and Callback instances
    /// can be added to the respective vectors, so we don't need the runtime
    /// type checking that the JavaScript version has.
    pub fn new(conditions: Vec<Condition>, callbacks: Vec<Callback>) -> Self {
        Self {
            conditions,
            callbacks,
        }
    }

    /// Create an empty Rule
    ///
    /// Equivalent to new Rule({}) in JavaScript
    pub fn empty() -> Self {
        Self {
            conditions: Vec::new(),
            callbacks: Vec::new(),
        }
    }

    /// Add a condition to the rule
    ///
    /// Equivalent to setting comparison property in JavaScript
    ///
    /// # Arguments
    ///
    /// * `condition` - Condition to add
    pub fn add_condition(&mut self, condition: Condition) {
        self.conditions.push(condition);
    }

    /// Add a condition from a JSON object
    ///
    /// Equivalent to setting comparison from object in JavaScript
    ///
    /// # Arguments
    ///
    /// * `condition_object` - JSON object representing a condition
    ///
    /// # Errors
    ///
    /// Returns error if the object cannot be converted to a Condition
    pub fn add_condition_from_object(&mut self, condition_object: &Value) -> Result<()> {
        let condition = Condition::from_object(condition_object)?;
        self.add_condition(condition);
        Ok(())
    }

    /// Add a callback to the rule
    ///
    /// Equivalent to setting callback property in JavaScript
    ///
    /// # Arguments
    ///
    /// * `callback` - Callback to add
    pub fn add_callback(&mut self, callback: Callback) {
        self.callbacks.push(callback);
    }

    /// Add a callback from a JSON object
    ///
    /// Equivalent to setting callback from object in JavaScript
    ///
    /// # Arguments
    ///
    /// * `callback_object` - JSON object representing a callback
    ///
    /// # Errors
    ///
    /// Returns error if the object cannot be converted to a Callback
    pub fn add_callback_from_object(&mut self, callback_object: &Value) -> Result<()> {
        let callback = Callback::from_object(callback_object)?;
        self.add_callback(callback);
        Ok(())
    }

    /// Create a Rule from a JSON object
    ///
    /// Equivalent to Rule.toObject() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `object` - JSON object containing rule data
    ///
    /// # Returns
    ///
    /// New Rule instance
    ///
    /// # Errors
    ///
    /// Returns error if condition or callback fields are missing
    pub fn from_object(object: &Value) -> Result<Self> {
        // Check for required fields (matching JS validation)
        let condition_array = object.get("condition")
            .ok_or_else(|| RuleArgumentError::new(
                "Rule::from_object() - Incorrect rule format! There is no condition field."
            ))?;

        let callback_array = object.get("callback")
            .ok_or_else(|| RuleArgumentError::new(
                "Rule::from_object() - Incorrect rule format! There is no callback field."
            ))?;

        let mut rule = Rule::empty();

        // Process conditions
        if let Some(conditions) = condition_array.as_array() {
            for condition_obj in conditions {
                rule.add_condition_from_object(condition_obj)?;
            }
        } else {
            return Err(RuleArgumentError::new(
                "Rule::from_object() - condition field must be an array"
            ).into());
        }

        // Process callbacks
        if let Some(callbacks) = callback_array.as_array() {
            for callback_obj in callbacks {
                rule.add_callback_from_object(callback_obj)?;
            }
        } else {
            return Err(RuleArgumentError::new(
                "Rule::from_object() - callback field must be an array"
            ).into());
        }

        Ok(rule)
    }

    /// Convert to JSON object
    ///
    /// Equivalent to toJSON() in JavaScript
    ///
    /// # Returns
    ///
    /// JSON representation of the rule
    pub fn to_json(&self) -> Value {
        let condition_values: Vec<Value> = self.conditions
            .iter()
            .map(|c| c.to_json())
            .collect();

        let callback_values: Vec<Value> = self.callbacks
            .iter()
            .map(|c| c.to_json())
            .collect();

        serde_json::json!({
            "condition": condition_values,
            "callback": callback_values
        })
    }

    /// Get the number of conditions
    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    /// Get the number of callbacks
    pub fn callback_count(&self) -> usize {
        self.callbacks.len()
    }

    /// Check if the rule is empty (no conditions and no callbacks)
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty() && self.callbacks.is_empty()
    }

    /// Get a reference to the conditions
    pub fn get_conditions(&self) -> &[Condition] {
        &self.conditions
    }

    /// Get a reference to the callbacks
    pub fn get_callbacks(&self) -> &[Callback] {
        &self.callbacks
    }

    /// Clear all conditions
    pub fn clear_conditions(&mut self) {
        self.conditions.clear();
    }

    /// Clear all callbacks
    pub fn clear_callbacks(&mut self) {
        self.callbacks.clear();
    }

    /// Clear both conditions and callbacks
    pub fn clear(&mut self) {
        self.clear_conditions();
        self.clear_callbacks();
    }
}

impl FromJsonObject<Rule> for Rule {
    fn from_json_object(object: &Value) -> Result<Rule> {
        Self::from_object(object)
    }
}

impl ToJsonObject for Rule {
    fn to_json_object(&self) -> Value {
        self.to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rule_new() {
        let condition = Condition::new(
            "balance".to_string(),
            "100".to_string(),
            ">=".to_string()
        ).unwrap();

        let callback = Callback::new(
            "collect".to_string(),
            None,
            None,
            None,
            Some("address".to_string()),
            Some("TOKEN".to_string()),
            Some("50".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        let rule = Rule::new(vec![condition], vec![callback]);

        assert_eq!(rule.condition_count(), 1);
        assert_eq!(rule.callback_count(), 1);
        assert!(!rule.is_empty());
    }

    #[test]
    fn test_rule_empty() {
        let rule = Rule::empty();

        assert_eq!(rule.condition_count(), 0);
        assert_eq!(rule.callback_count(), 0);
        assert!(rule.is_empty());
    }

    #[test]
    fn test_rule_add_condition() {
        let mut rule = Rule::empty();

        let condition = Condition::new(
            "token".to_string(),
            "TEST".to_string(),
            "==".to_string()
        ).unwrap();

        rule.add_condition(condition);

        assert_eq!(rule.condition_count(), 1);
        assert_eq!(rule.get_conditions()[0].key, "token");
        assert_eq!(rule.get_conditions()[0].value, "TEST");
        assert_eq!(rule.get_conditions()[0].comparison, "==");
    }

    #[test]
    fn test_rule_add_callback() {
        let mut rule = Rule::empty();

        let callback = Callback::new(
            "reject".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();

        rule.add_callback(callback);

        assert_eq!(rule.callback_count(), 1);
        assert_eq!(rule.get_callbacks()[0].action, "reject");
    }

    #[test]
    fn test_rule_add_condition_from_object() {
        let mut rule = Rule::empty();

        let condition_object = json!({
            "key": "amount",
            "value": "1000",
            "comparison": "<"
        });

        rule.add_condition_from_object(&condition_object).unwrap();

        assert_eq!(rule.condition_count(), 1);
        assert_eq!(rule.get_conditions()[0].key, "amount");
        assert_eq!(rule.get_conditions()[0].value, "1000");
        assert_eq!(rule.get_conditions()[0].comparison, "<");
    }

    #[test]
    fn test_rule_add_callback_from_object() {
        let mut rule = Rule::empty();

        let callback_object = json!({
            "action": "burn",
            "token": "FIRE",
            "amount": "100",
            "comparison": ">"
        });

        rule.add_callback_from_object(&callback_object).unwrap();

        assert_eq!(rule.callback_count(), 1);
        assert_eq!(rule.get_callbacks()[0].action, "burn");
        assert_eq!(rule.get_callbacks()[0].token, Some("FIRE".to_string()));
        assert_eq!(rule.get_callbacks()[0].amount, Some("100".to_string()));
        assert_eq!(rule.get_callbacks()[0].comparison, Some(">".to_string()));
    }

    #[test]
    fn test_rule_from_object() {
        let object = json!({
            "condition": [
                {
                    "key": "balance",
                    "value": "500",
                    "comparison": ">="
                },
                {
                    "key": "token",
                    "value": "TEST",
                    "comparison": "=="
                }
            ],
            "callback": [
                {
                    "action": "collect",
                    "address": "test_address",
                    "token": "TEST",
                    "amount": "100",
                    "comparison": ">="
                },
                {
                    "action": "reject"
                }
            ]
        });

        let rule = Rule::from_object(&object).unwrap();

        assert_eq!(rule.condition_count(), 2);
        assert_eq!(rule.callback_count(), 2);

        // Check first condition
        assert_eq!(rule.get_conditions()[0].key, "balance");
        assert_eq!(rule.get_conditions()[0].value, "500");
        assert_eq!(rule.get_conditions()[0].comparison, ">=");

        // Check second condition
        assert_eq!(rule.get_conditions()[1].key, "token");
        assert_eq!(rule.get_conditions()[1].value, "TEST");
        assert_eq!(rule.get_conditions()[1].comparison, "==");

        // Check first callback
        assert_eq!(rule.get_callbacks()[0].action, "collect");
        assert_eq!(rule.get_callbacks()[0].address, Some("test_address".to_string()));
        assert_eq!(rule.get_callbacks()[0].token, Some("TEST".to_string()));
        assert_eq!(rule.get_callbacks()[0].amount, Some("100".to_string()));
        assert_eq!(rule.get_callbacks()[0].comparison, Some(">=".to_string()));

        // Check second callback
        assert_eq!(rule.get_callbacks()[1].action, "reject");
    }

    #[test]
    fn test_rule_from_object_missing_condition() {
        let object = json!({
            "callback": [
                {
                    "action": "reject"
                }
            ]
        });

        let result = Rule::from_object(&object);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no condition field"));
    }

    #[test]
    fn test_rule_from_object_missing_callback() {
        let object = json!({
            "condition": [
                {
                    "key": "balance",
                    "value": "500",
                    "comparison": ">="
                }
            ]
        });

        let result = Rule::from_object(&object);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no callback field"));
    }

    #[test]
    fn test_rule_from_object_invalid_condition_type() {
        let object = json!({
            "condition": "not_an_array",
            "callback": []
        });

        let result = Rule::from_object(&object);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("condition field must be an array"));
    }

    #[test]
    fn test_rule_from_object_invalid_callback_type() {
        let object = json!({
            "condition": [],
            "callback": "not_an_array"
        });

        let result = Rule::from_object(&object);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("callback field must be an array"));
    }

    #[test]
    fn test_rule_to_json() {
        let condition1 = Condition::new(
            "balance".to_string(),
            "100".to_string(),
            ">=".to_string()
        ).unwrap();

        let condition2 = Condition::new(
            "token".to_string(),
            "TEST".to_string(),
            "==".to_string()
        ).unwrap();

        let callback1 = Callback::new(
            "collect".to_string(),
            None,
            None,
            None,
            Some("address".to_string()),
            Some("TOKEN".to_string()),
            Some("50".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        let callback2 = Callback::new(
            "reject".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();

        let rule = Rule::new(vec![condition1, condition2], vec![callback1, callback2]);
        let json_value = rule.to_json();

        // Check conditions
        let conditions = json_value["condition"].as_array().unwrap();
        assert_eq!(conditions.len(), 2);
        assert_eq!(conditions[0]["key"], "balance");
        assert_eq!(conditions[0]["value"], "100");
        assert_eq!(conditions[0]["comparison"], ">=");
        assert_eq!(conditions[1]["key"], "token");
        assert_eq!(conditions[1]["value"], "TEST");
        assert_eq!(conditions[1]["comparison"], "==");

        // Check callbacks
        let callbacks = json_value["callback"].as_array().unwrap();
        assert_eq!(callbacks.len(), 2);
        assert_eq!(callbacks[0]["action"], "collect");
        assert_eq!(callbacks[0]["address"], "address");
        assert_eq!(callbacks[0]["token"], "TOKEN");
        assert_eq!(callbacks[0]["amount"], "50");
        assert_eq!(callbacks[0]["comparison"], ">=");
        assert_eq!(callbacks[1]["action"], "reject");
    }

    #[test]
    fn test_rule_clear_operations() {
        let condition = Condition::new(
            "test".to_string(),
            "value".to_string(),
            "==".to_string()
        ).unwrap();

        let callback = Callback::new(
            "test_action".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();

        let mut rule = Rule::new(vec![condition], vec![callback]);

        assert_eq!(rule.condition_count(), 1);
        assert_eq!(rule.callback_count(), 1);
        assert!(!rule.is_empty());

        rule.clear_conditions();
        assert_eq!(rule.condition_count(), 0);
        assert_eq!(rule.callback_count(), 1);

        rule.clear_callbacks();
        assert_eq!(rule.condition_count(), 0);
        assert_eq!(rule.callback_count(), 0);
        assert!(rule.is_empty());

        // Test clear all
        let condition2 = Condition::new(
            "test2".to_string(),
            "value2".to_string(),
            "!=".to_string()
        ).unwrap();

        let callback2 = Callback::new(
            "test_action2".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).unwrap();

        rule.add_condition(condition2);
        rule.add_callback(callback2);

        assert!(!rule.is_empty());
        rule.clear();
        assert!(rule.is_empty());
    }

    #[test]
    fn test_rule_serialization() {
        let condition = Condition::new(
            "amount".to_string(),
            "1000".to_string(),
            "<=".to_string()
        ).unwrap();

        let callback = Callback::new(
            "buffer".to_string(),
            None,
            None,
            None,
            Some("buffer_address".to_string()),
            Some("BUFFER".to_string()),
            Some("200".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        let rule = Rule::new(vec![condition], vec![callback]);

        // Test JSON serialization
        let json_str = serde_json::to_string(&rule).unwrap();
        let deserialized: Rule = serde_json::from_str(&json_str).unwrap();

        assert_eq!(rule, deserialized);
        assert_eq!(deserialized.condition_count(), 1);
        assert_eq!(deserialized.callback_count(), 1);
    }

    #[test]
    fn test_rule_traits() {
        let object = json!({
            "condition": [
                {
                    "key": "status",
                    "value": "active",
                    "comparison": "=="
                }
            ],
            "callback": [
                {
                    "action": "remit",
                    "token": "REMIT",
                    "amount": "75"
                }
            ]
        });

        // Test FromJsonObject trait
        let rule = Rule::from_json_object(&object).unwrap();
        assert_eq!(rule.condition_count(), 1);
        assert_eq!(rule.callback_count(), 1);
        assert_eq!(rule.get_conditions()[0].key, "status");
        assert_eq!(rule.get_conditions()[0].value, "active");
        assert_eq!(rule.get_callbacks()[0].action, "remit");

        // Test ToJsonObject trait
        let json_output = rule.to_json_object();
        assert!(json_output["condition"].is_array());
        assert!(json_output["callback"].is_array());
        assert_eq!(json_output["condition"][0]["key"], "status");
        assert_eq!(json_output["callback"][0]["action"], "remit");
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test that our implementation matches JavaScript SDK patterns

        // Test empty rule creation (matches JS new Rule({}))
        let empty_rule = Rule::empty();
        assert!(empty_rule.is_empty());

        // Test rule creation with arrays (matches JS constructor)
        let condition = Condition::new(
            "balance".to_string(),
            "500".to_string(),
            ">".to_string()
        ).unwrap();

        let callback = Callback::new(
            "collect".to_string(),
            None,
            None,
            None,
            Some("test_address".to_string()),
            Some("TEST".to_string()),
            Some("100".to_string()),
            Some(">=".to_string()),
        ).unwrap();

        let rule = Rule::new(vec![condition], vec![callback]);
        assert_eq!(rule.condition_count(), 1);
        assert_eq!(rule.callback_count(), 1);

        // Test JS-style object creation (matches JS Rule.toObject method)
        let js_style_object = json!({
            "condition": [
                {
                    "key": "wallet_balance",
                    "value": "1000",
                    "comparison": ">="
                }
            ],
            "callback": [
                {
                    "action": "collect",
                    "address": "collect_address",
                    "token": "COLLECT",
                    "amount": "250",
                    "comparison": ">="
                }
            ]
        });

        let rule_from_object = Rule::from_object(&js_style_object).unwrap();
        let json_output = rule_from_object.to_json();

        // Verify round-trip compatibility
        assert_eq!(json_output["condition"][0]["key"], js_style_object["condition"][0]["key"]);
        assert_eq!(json_output["condition"][0]["value"], js_style_object["condition"][0]["value"]);
        assert_eq!(json_output["condition"][0]["comparison"], js_style_object["condition"][0]["comparison"]);
        assert_eq!(json_output["callback"][0]["action"], js_style_object["callback"][0]["action"]);
        assert_eq!(json_output["callback"][0]["address"], js_style_object["callback"][0]["address"]);

        // Test setter-style addition (matches JS property setters)
        let mut dynamic_rule = Rule::empty();
        
        let new_condition = Condition::new(
            "token_type".to_string(),
            "DYNAMIC".to_string(),
            "==".to_string()
        ).unwrap();
        
        let new_callback = Callback::new(
            "burn".to_string(),
            None,
            None,
            None,
            None,
            Some("BURN".to_string()),
            Some("50".to_string()),
            Some(">".to_string()),
        ).unwrap();

        dynamic_rule.add_condition(new_condition);
        dynamic_rule.add_callback(new_callback);

        assert_eq!(dynamic_rule.condition_count(), 1);
        assert_eq!(dynamic_rule.callback_count(), 1);
        assert_eq!(dynamic_rule.get_conditions()[0].key, "token_type");
        assert_eq!(dynamic_rule.get_callbacks()[0].action, "burn");
    }
}