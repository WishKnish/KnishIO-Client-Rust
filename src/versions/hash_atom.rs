//! HashAtom base implementation
//!
//! Equivalent to HashAtom.js in the JavaScript SDK

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::atom::Atom;
use crate::error::Result;
use super::{StructureUtils, AtomVersion};

/// Base class for atom hashing implementations
///
/// Equivalent to HashAtom class in JavaScript SDK
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HashAtom {
    // This is the base class - specific versions will extend this
}

impl HashAtom {
    /// Create a new HashAtom instance
    pub fn new() -> Self {
        Self {}
    }

    /// Create a HashAtom-based object from an Atom
    ///
    /// Equivalent to HashAtom.create() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `atom` - The atom to create from
    ///
    /// # Returns
    ///
    /// A new HashAtom instance with the atom's data
    ///
    /// # Note
    ///
    /// This is a generic implementation. Specific versions should override this.
    pub fn create_from_atom(_atom: &Atom) -> Self {
        // Base implementation - specific versions will have their own fields
        Self::new()
    }

    /// Convert object to structured format for hashing
    ///
    /// Equivalent to HashAtom.structure() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `object` - The object to structure
    ///
    /// # Returns
    ///
    /// Structured representation of the object
    pub fn structure(object: &Value) -> Value {
        StructureUtils::structure(object)
    }

    /// Check if a value is a complex structure
    ///
    /// Equivalent to HashAtom.isStructure() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `structure` - The value to check
    ///
    /// # Returns
    ///
    /// True if the value is an object or array
    pub fn is_structure(structure: &Value) -> bool {
        StructureUtils::is_structure(structure)
    }

    /// Get the structured view of this object
    ///
    /// Equivalent to view() in JavaScript
    ///
    /// # Returns
    ///
    /// Structured representation of this object
    pub fn view(&self) -> Value {
        let value = serde_json::to_value(self).unwrap_or(Value::Null);
        Self::structure(&value)
    }

    /// Create a structured representation from any serializable data
    ///
    /// Helper method for creating consistent structured views
    pub fn create_view<T: Serialize>(data: &T) -> Result<Value> {
        let value = serde_json::to_value(data)?;
        Ok(Self::structure(&value))
    }
}

impl Default for HashAtom {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomVersion for HashAtom {
    fn from_atom(atom: &Atom) -> Self {
        Self::create_from_atom(atom)
    }
    
    fn view(&self) -> Value {
        self.view()
    }
    
    fn version(&self) -> String {
        "base".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::types::Isotope;

    #[test]
    fn test_hash_atom_new() {
        let hash_atom = HashAtom::new();
        assert_eq!(hash_atom, HashAtom::default());
    }

    #[test]
    fn test_create_from_atom() {
        let atom = Atom::new(
            "W1",
            "test-address",
            Isotope::V,
            "TEST"
        );

        let hash_atom = HashAtom::create_from_atom(&atom);
        assert_eq!(hash_atom, HashAtom::new());
    }

    #[test]
    fn test_structure_static_method() {
        let object = json!({
            "z_key": "value1",
            "a_key": "value2"
        });

        let structured = HashAtom::structure(&object);
        
        // Should convert to array with sorted keys
        assert!(structured.is_array());
        let arr = structured.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        
        // Keys should be sorted alphabetically
        assert_eq!(arr[0], json!({"a_key": "value2"}));
        assert_eq!(arr[1], json!({"z_key": "value1"}));
    }

    #[test]
    fn test_is_structure_static_method() {
        // Test objects and arrays
        assert!(HashAtom::is_structure(&json!({"key": "value"})));
        assert!(HashAtom::is_structure(&json!(["item1", "item2"])));
        
        // Test primitives
        assert!(!HashAtom::is_structure(&json!("string")));
        assert!(!HashAtom::is_structure(&json!(123)));
        assert!(!HashAtom::is_structure(&json!(true)));
        assert!(!HashAtom::is_structure(&Value::Null));
    }

    #[test]
    fn test_view() {
        let hash_atom = HashAtom::new();
        let view = hash_atom.view();
        
        // View should be structured representation of empty object
        assert!(view.is_array() || view.is_object());
    }

    #[test]
    fn test_create_view() {
        #[derive(Serialize)]
        struct TestData {
            field1: String,
            field2: i32,
        }

        let data = TestData {
            field1: "test".to_string(),
            field2: 42,
        };

        let view = HashAtom::create_view(&data).unwrap();
        
        // Should create structured representation
        assert!(view.is_array());
        let arr = view.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        
        // Fields should be sorted
        assert!(arr[0].as_object().unwrap().contains_key("field1"));
        assert!(arr[1].as_object().unwrap().contains_key("field2"));
    }

    #[test]
    fn test_atom_version_trait() {
        let atom = Atom::new(
            "W1",
            "test-address",
            Isotope::C,
            "TEST"
        );

        let hash_atom = HashAtom::from_atom(&atom);
        assert_eq!(hash_atom.version(), "base");
        
        let view = hash_atom.view();
        assert!(view.is_array() || view.is_object());
    }

    #[test]
    fn test_structure_complex_object() {
        let complex_object = json!({
            "level1": {
                "nested": {
                    "deep": "value"
                },
                "array": ["item1", "item2"]
            },
            "simple": "value",
            "number": 42
        });

        let structured = HashAtom::structure(&complex_object);
        
        // Should convert to array with sorted keys
        assert!(structured.is_array());
        let arr = structured.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        
        // Keys should be sorted: level1, number, simple
        assert!(arr[0].as_object().unwrap().contains_key("level1"));
        assert!(arr[1].as_object().unwrap().contains_key("number"));
        assert!(arr[2].as_object().unwrap().contains_key("simple"));
        
        // Nested objects should also be structured
        let level1_value = arr[0].as_object().unwrap().get("level1").unwrap();
        assert!(level1_value.is_array()); // Nested object becomes array
    }

    #[test]
    fn test_structure_array_with_objects() {
        let array_with_objects = json!([
            {"z_key": "value1"},
            "string_item",
            {"a_key": "value2"},
            123
        ]);

        let structured = HashAtom::structure(&array_with_objects);
        
        // Array should remain array but objects inside should be structured
        assert!(structured.is_array());
        let arr = structured.as_array().unwrap();
        assert_eq!(arr.len(), 4);
        
        // First object should be structured (as array)
        assert!(arr[0].is_array());
        
        // String should remain unchanged
        assert_eq!(arr[1], json!("string_item"));
        
        // Third object should be structured (as array)
        assert!(arr[2].is_array());
        
        // Number should remain unchanged
        assert_eq!(arr[3], json!(123));
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test that our implementation matches JavaScript SDK patterns
        
        // Test the create method equivalent
        let atom = Atom::new("W1", "addr", Isotope::V, "TOKEN");
        let hash_atom = HashAtom::create_from_atom(&atom);
        assert_eq!(hash_atom.version(), "base");
        
        // Test structure method matches JS behavior
        let js_object = json!({
            "position": "W1",
            "walletAddress": "addr",
            "isotope": "V",
            "token": "TOKEN"
        });
        
        let structured = HashAtom::structure(&js_object);
        assert!(structured.is_array());
        
        let arr = structured.as_array().unwrap();
        assert_eq!(arr.len(), 4);
        
        // Should be sorted: isotope, position, token, walletAddress
        assert!(arr[0].as_object().unwrap().contains_key("isotope"));
        assert!(arr[1].as_object().unwrap().contains_key("position"));
        assert!(arr[2].as_object().unwrap().contains_key("token"));
        assert!(arr[3].as_object().unwrap().contains_key("walletAddress"));
        
        // Test isStructure method
        assert!(HashAtom::is_structure(&json!({"key": "value"})));
        assert!(HashAtom::is_structure(&json!(["item"])));
        assert!(!HashAtom::is_structure(&json!("string")));
        
        // Test view method
        let view = hash_atom.view();
        assert!(view.is_array() || view.is_object());
    }
}