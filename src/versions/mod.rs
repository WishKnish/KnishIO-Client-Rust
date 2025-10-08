//! Version utilities for KnishIO DLT
//!
//! This module provides version-specific implementations for atom hashing
//! and structure serialization, matching the JavaScript SDK functionality.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use crate::atom::Atom;
use crate::error::Result;

pub mod hash_atom;
pub mod version4;

pub use hash_atom::HashAtom;
pub use version4::Version4;

/// Trait for version-specific atom implementations
pub trait AtomVersion {
    /// Create a version-specific atom from a generic Atom
    fn from_atom(atom: &Atom) -> Self;
    
    /// Convert to a structured view for hashing
    fn view(&self) -> Value;
    
    /// Get the version identifier
    fn version(&self) -> String;
}

/// Utility functions for structure manipulation
pub struct StructureUtils;

impl StructureUtils {
    /// Check if a value is a complex structure (object or array)
    ///
    /// Equivalent to HashAtom.isStructure() in JavaScript
    pub fn is_structure(value: &Value) -> bool {
        matches!(value, Value::Object(_) | Value::Array(_))
    }

    /// Convert any value to a structured format for consistent hashing
    ///
    /// Equivalent to HashAtom.structure() in JavaScript
    pub fn structure(value: &Value) -> Value {
        match value {
            Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    if Self::is_structure(item) {
                        result.push(Self::structure(item));
                    } else {
                        result.push(item.clone());
                    }
                }
                Value::Array(result)
            }
            Value::Object(obj) => {
                let mut result = Vec::new();
                
                // Sort keys for consistent ordering (like JavaScript sort)
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                
                for key in keys {
                    if let Some(value) = obj.get(key) {
                        let mut item = BTreeMap::new();
                        if Self::is_structure(value) {
                            item.insert(key.clone(), Self::structure(value));
                        } else {
                            item.insert(key.clone(), value.clone());
                        }
                        let map: serde_json::Map<String, Value> = item.into_iter().collect();
                        result.push(Value::Object(map));
                    }
                }
                
                if !result.is_empty() {
                    Value::Array(result)
                } else {
                    value.clone()
                }
            }
            _ => value.clone(),
        }
    }

    /// Create a sorted object representation from key-value pairs
    pub fn create_sorted_object<T: Serialize>(data: &T) -> Result<Value> {
        let value = serde_json::to_value(data)?;
        Ok(Self::structure(&value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_structure() {
        // Objects and arrays should be structures
        assert!(StructureUtils::is_structure(&json!({"key": "value"})));
        assert!(StructureUtils::is_structure(&json!(["item1", "item2"])));
        
        // Primitives should not be structures
        assert!(!StructureUtils::is_structure(&json!("string")));
        assert!(!StructureUtils::is_structure(&json!(123)));
        assert!(!StructureUtils::is_structure(&json!(true)));
        assert!(!StructureUtils::is_structure(&Value::Null));
    }

    #[test]
    fn test_structure_array() {
        let input = json!(["item1", {"nested": "value"}, "item3"]);
        let result = StructureUtils::structure(&input);
        
        // Should process nested objects but keep arrays as arrays
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], json!("item1"));
        assert!(arr[1].is_array()); // Nested object becomes array
        assert_eq!(arr[2], json!("item3"));
    }

    #[test]
    fn test_structure_object() {
        let input = json!({
            "z_last": "value1",
            "a_first": "value2",
            "nested": {
                "inner": "value3"
            }
        });
        
        let result = StructureUtils::structure(&input);
        
        // Should be converted to sorted array of key-value objects
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        
        // Keys should be sorted alphabetically
        assert_eq!(arr[0], json!({"a_first": "value2"}));
        assert!(arr[1].as_object().unwrap().contains_key("nested"));
        assert_eq!(arr[2], json!({"z_last": "value1"}));
    }

    #[test]
    fn test_structure_primitives() {
        // Primitives should pass through unchanged
        assert_eq!(StructureUtils::structure(&json!("test")), json!("test"));
        assert_eq!(StructureUtils::structure(&json!(123)), json!(123));
        assert_eq!(StructureUtils::structure(&json!(true)), json!(true));
        assert_eq!(StructureUtils::structure(&Value::Null), Value::Null);
    }

    #[test]
    fn test_structure_nested_complex() {
        let input = json!({
            "level1": {
                "level2": {
                    "level3": ["item1", "item2"]
                },
                "sibling": "value"
            },
            "array": [
                {"object_in_array": "value"},
                "primitive"
            ]
        });
        
        let result = StructureUtils::structure(&input);
        
        // Complex nesting should be handled recursively
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        
        // Check that keys are sorted and structure is maintained
        assert!(arr[0].as_object().unwrap().contains_key("array"));
        assert!(arr[1].as_object().unwrap().contains_key("level1"));
    }

    #[test]
    fn test_create_sorted_object() {
        #[derive(Serialize)]
        struct TestStruct {
            z_field: String,
            a_field: i32,
            nested: TestNested,
        }
        
        #[derive(Serialize)]
        struct TestNested {
            inner: String,
        }
        
        let test_data = TestStruct {
            z_field: "last".to_string(),
            a_field: 42,
            nested: TestNested {
                inner: "value".to_string(),
            },
        };
        
        let result = StructureUtils::create_sorted_object(&test_data).unwrap();
        
        // Should create a structured representation
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        
        // Keys should be alphabetically sorted
        assert!(arr[0].as_object().unwrap().contains_key("a_field"));
        assert!(arr[1].as_object().unwrap().contains_key("nested"));
        assert!(arr[2].as_object().unwrap().contains_key("z_field"));
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test that our implementation matches JavaScript SDK patterns
        
        // Test object key sorting (matches JS sort behavior)
        let js_style_object = json!({
            "zebra": 1,
            "alpha": 2,
            "beta": 3
        });
        
        let structured = StructureUtils::structure(&js_style_object);
        let array = structured.as_array().unwrap();
        
        // Should be sorted: alpha, beta, zebra
        assert_eq!(array[0], json!({"alpha": 2}));
        assert_eq!(array[1], json!({"beta": 3}));
        assert_eq!(array[2], json!({"zebra": 1}));
        
        // Test array preservation
        let js_array = json!(["item1", "item2", "item3"]);
        let structured_array = StructureUtils::structure(&js_array);
        assert_eq!(structured_array, js_array); // Arrays should remain unchanged
        
        // Test nested structure handling
        let nested = json!({
            "outer": {
                "inner": ["a", "b", "c"]
            }
        });
        
        let structured_nested = StructureUtils::structure(&nested);
        assert!(structured_nested.is_array());
        
        // Verify that nested objects are also converted to arrays
        let outer_item = &structured_nested.as_array().unwrap()[0];
        let outer_value = outer_item.as_object().unwrap().get("outer").unwrap();
        assert!(outer_value.is_array()); // Inner object should be array format
    }
}