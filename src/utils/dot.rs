//! Dot notation utility for nested object access
//!
//! This module provides utilities for accessing nested object properties using dot notation,
//! ensuring exact compatibility with the JavaScript SDK's Dot.js implementation.

use serde_json::Value;
use std::collections::HashMap;

/// Dot notation utility class
///
/// Equivalent to Dot.js in the JavaScript SDK, this provides utilities for accessing
/// nested object properties using dot notation like "foo.bar.baz".
pub struct Dot;

impl Dot {
    /// Check if a nested property exists in a JSON value using dot notation
    ///
    /// Equivalent to Dot.has() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON value to search
    /// * `keys` - The path to the property, using dot notation
    ///
    /// # Returns
    ///
    /// `true` if the property exists, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::Dot;
    /// use serde_json::json;
    ///
    /// let data = json!({
    ///     "user": {
    ///         "profile": {
    ///             "name": "John"
    ///         }
    ///     }
    /// });
    ///
    /// assert!(Dot::has(&data, "user.profile.name"));
    /// assert!(!Dot::has(&data, "user.profile.age"));
    /// ```
    pub fn has(obj: &Value, keys: &str) -> bool {
        let parts: Vec<&str> = keys.split('.').collect();
        Self::has_recursive(obj, &parts)
    }

    /// Get a nested property from a JSON value using dot notation
    ///
    /// Equivalent to Dot.get() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON value to search
    /// * `keys` - The path to the property, using dot notation
    /// * `default` - Optional default value to return if property is not found
    ///
    /// # Returns
    ///
    /// The value of the property, or the default value if not found
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::Dot;
    /// use serde_json::{json, Value};
    ///
    /// let data = json!({
    ///     "user": {
    ///         "profile": {
    ///             "name": "John",
    ///             "age": 30
    ///         }
    ///     }
    /// });
    ///
    /// let name = Dot::get(&data, "user.profile.name", None);
    /// assert_eq!(name, Some(&json!("John")));
    ///
    /// let missing = Dot::get(&data, "user.profile.email", None);
    /// assert_eq!(missing, None);
    /// ```
    pub fn get<'a>(obj: &'a Value, keys: &str, default: Option<&'a Value>) -> Option<&'a Value> {
        let parts: Vec<&str> = keys.split('.').collect();
        Self::get_recursive(obj, &parts).or(default)
    }

    /// Set a nested property in a JSON value using dot notation
    ///
    /// Equivalent to Dot.set() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `obj` - The mutable JSON value to modify
    /// * `keys` - The path to the property, using dot notation
    /// * `value` - The value to set
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::Dot;
    /// use serde_json::{json, Value};
    ///
    /// let mut data = json!({});
    /// Dot::set(&mut data, "user.profile.name", json!("John"));
    ///
    /// assert_eq!(data["user"]["profile"]["name"], "John");
    /// ```
    pub fn set(obj: &mut Value, keys: &str, value: Value) {
        let parts: Vec<&str> = keys.split('.').collect();
        Self::set_recursive(obj, &parts, value);
    }

    /// Get a string value using dot notation (convenience method)
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON value to search
    /// * `keys` - The path to the property, using dot notation
    ///
    /// # Returns
    ///
    /// Optional string value
    pub fn get_string(obj: &Value, keys: &str) -> Option<String> {
        Self::get(obj, keys, None)?.as_str().map(|s| s.to_string())
    }

    /// Get a number value using dot notation (convenience method)
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON value to search
    /// * `keys` - The path to the property, using dot notation
    ///
    /// # Returns
    ///
    /// Optional f64 value
    pub fn get_number(obj: &Value, keys: &str) -> Option<f64> {
        Self::get(obj, keys, None)?.as_f64()
    }

    /// Get a boolean value using dot notation (convenience method)
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON value to search
    /// * `keys` - The path to the property, using dot notation
    ///
    /// # Returns
    ///
    /// Optional boolean value
    pub fn get_bool(obj: &Value, keys: &str) -> Option<bool> {
        Self::get(obj, keys, None)?.as_bool()
    }

    /// Get an array value using dot notation (convenience method)
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON value to search
    /// * `keys` - The path to the property, using dot notation
    ///
    /// # Returns
    ///
    /// Optional array reference
    pub fn get_array<'a>(obj: &'a Value, keys: &str) -> Option<&'a Vec<Value>> {
        Self::get(obj, keys, None)?.as_array()
    }

    /// Get an object value using dot notation (convenience method)
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON value to search
    /// * `keys` - The path to the property, using dot notation
    ///
    /// # Returns
    ///
    /// Optional object reference
    pub fn get_object<'a>(obj: &'a Value, keys: &str) -> Option<&'a serde_json::Map<String, Value>> {
        Self::get(obj, keys, None)?.as_object()
    }

    /// Helper method for HashMap access using dot notation
    ///
    /// # Arguments
    ///
    /// * `map` - The HashMap to search
    /// * `keys` - The path to the property, using dot notation
    ///
    /// # Returns
    ///
    /// Optional string value (owned for simplicity)
    pub fn get_from_map(map: &HashMap<String, String>, keys: &str) -> Option<String> {
        // Simple implementation for single key access
        if !keys.contains('.') {
            return map.get(keys).cloned();
        }
        
        // For nested access, convert to JSON and use regular get
        if let Ok(json_value) = serde_json::to_value(map) {
            Self::get_string(&json_value, keys)
        } else {
            None
        }
    }

    // Private helper methods

    /// Recursive helper for has() method
    fn has_recursive(obj: &Value, parts: &[&str]) -> bool {
        if parts.is_empty() {
            return true;
        }

        let key = parts[0];
        let remaining = &parts[1..];

        // Try array index access
        if let Ok(index) = key.parse::<usize>() {
            if let Some(array) = obj.as_array() {
                if let Some(value) = array.get(index) {
                    return Self::has_recursive(value, remaining);
                }
            }
        }

        // Try object key access
        if let Some(object) = obj.as_object() {
            if let Some(value) = object.get(key) {
                return Self::has_recursive(value, remaining);
            }
        }

        false
    }

    /// Recursive helper for get() method
    fn get_recursive<'a>(obj: &'a Value, parts: &[&str]) -> Option<&'a Value> {
        if parts.is_empty() {
            return Some(obj);
        }

        let key = parts[0];
        let remaining = &parts[1..];

        // Try array index access
        if let Ok(index) = key.parse::<usize>() {
            if let Some(array) = obj.as_array() {
                if let Some(value) = array.get(index) {
                    return Self::get_recursive(value, remaining);
                }
            }
        }

        // Try object key access
        if let Some(object) = obj.as_object() {
            if let Some(value) = object.get(key) {
                return Self::get_recursive(value, remaining);
            }
        }

        None
    }

    /// Recursive helper for set() method
    fn set_recursive(obj: &mut Value, parts: &[&str], value: Value) {
        if parts.is_empty() {
            return;
        }

        let key = parts[0];
        let remaining = &parts[1..];

        if remaining.is_empty() {
            // Last part - set the value
            if let Ok(index) = key.parse::<usize>() {
                // Array index
                if let Some(array) = obj.as_array_mut() {
                    // Extend array if necessary
                    while array.len() <= index {
                        array.push(Value::Null);
                    }
                    array[index] = value;
                } else {
                    // Convert to array
                    let mut new_array = vec![Value::Null; index + 1];
                    new_array[index] = value;
                    *obj = Value::Array(new_array);
                }
            } else {
                // Object key
                if let Some(object) = obj.as_object_mut() {
                    object.insert(key.to_string(), value);
                } else {
                    // Convert to object
                    let mut new_object = serde_json::Map::new();
                    new_object.insert(key.to_string(), value);
                    *obj = Value::Object(new_object);
                }
            }
        } else {
            // Intermediate part - ensure path exists
            if let Ok(index) = key.parse::<usize>() {
                // Array index
                if let Some(array) = obj.as_array_mut() {
                    // Extend array if necessary
                    while array.len() <= index {
                        array.push(Value::Null);
                    }
                    if array[index].is_null() {
                        // Determine if next key is numeric
                        if remaining[0].parse::<usize>().is_ok() {
                            array[index] = Value::Array(vec![]);
                        } else {
                            array[index] = Value::Object(serde_json::Map::new());
                        }
                    }
                    Self::set_recursive(&mut array[index], remaining, value);
                } else {
                    // Convert to array
                    let mut new_array = vec![Value::Null; index + 1];
                    if remaining[0].parse::<usize>().is_ok() {
                        new_array[index] = Value::Array(vec![]);
                    } else {
                        new_array[index] = Value::Object(serde_json::Map::new());
                    }
                    *obj = Value::Array(new_array);
                    if let Some(array) = obj.as_array_mut() {
                        Self::set_recursive(&mut array[index], remaining, value);
                    }
                }
            } else {
                // Object key
                if let Some(object) = obj.as_object_mut() {
                    if !object.contains_key(key) {
                        // Determine if next key is numeric
                        if remaining[0].parse::<usize>().is_ok() {
                            object.insert(key.to_string(), Value::Array(vec![]));
                        } else {
                            object.insert(key.to_string(), Value::Object(serde_json::Map::new()));
                        }
                    }
                    if let Some(nested) = object.get_mut(key) {
                        Self::set_recursive(nested, remaining, value);
                    }
                } else {
                    // Convert to object
                    let mut new_object = serde_json::Map::new();
                    if remaining[0].parse::<usize>().is_ok() {
                        new_object.insert(key.to_string(), Value::Array(vec![]));
                    } else {
                        new_object.insert(key.to_string(), Value::Object(serde_json::Map::new()));
                    }
                    *obj = Value::Object(new_object);
                    if let Some(object) = obj.as_object_mut() {
                        if let Some(nested) = object.get_mut(key) {
                            Self::set_recursive(nested, remaining, value);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_has() {
        let data = json!({
            "user": {
                "profile": {
                    "name": "John",
                    "settings": {
                        "theme": "dark"
                    }
                },
                "posts": [
                    {"title": "First Post"},
                    {"title": "Second Post"}
                ]
            }
        });

        assert!(Dot::has(&data, "user"));
        assert!(Dot::has(&data, "user.profile"));
        assert!(Dot::has(&data, "user.profile.name"));
        assert!(Dot::has(&data, "user.profile.settings.theme"));
        assert!(Dot::has(&data, "user.posts"));
        assert!(Dot::has(&data, "user.posts.0"));
        assert!(Dot::has(&data, "user.posts.0.title"));
        assert!(Dot::has(&data, "user.posts.1.title"));

        assert!(!Dot::has(&data, "user.profile.age"));
        assert!(!Dot::has(&data, "user.posts.2"));
        assert!(!Dot::has(&data, "nonexistent"));
    }

    #[test]
    fn test_get() {
        let data = json!({
            "user": {
                "name": "John",
                "age": 30,
                "active": true,
                "tags": ["developer", "rust"]
            }
        });

        assert_eq!(Dot::get(&data, "user.name", None), Some(&json!("John")));
        assert_eq!(Dot::get(&data, "user.age", None), Some(&json!(30)));
        assert_eq!(Dot::get(&data, "user.active", None), Some(&json!(true)));
        assert_eq!(Dot::get(&data, "user.tags.0", None), Some(&json!("developer")));
        assert_eq!(Dot::get(&data, "user.tags.1", None), Some(&json!("rust")));
        assert_eq!(Dot::get(&data, "user.nonexistent", None), None);

        let default = json!("default");
        assert_eq!(Dot::get(&data, "user.nonexistent", Some(&default)), Some(&default));
    }

    #[test]
    fn test_set() {
        let mut data = json!({});

        Dot::set(&mut data, "user.name", json!("John"));
        assert_eq!(data["user"]["name"], "John");

        Dot::set(&mut data, "user.age", json!(30));
        assert_eq!(data["user"]["age"], 30);

        Dot::set(&mut data, "user.tags.0", json!("developer"));
        assert_eq!(data["user"]["tags"][0], "developer");

        Dot::set(&mut data, "user.tags.1", json!("rust"));
        assert_eq!(data["user"]["tags"][1], "rust");

        Dot::set(&mut data, "config.settings.theme", json!("dark"));
        assert_eq!(data["config"]["settings"]["theme"], "dark");
    }

    #[test]
    fn test_convenience_methods() {
        let data = json!({
            "user": {
                "name": "John",
                "age": 30,
                "active": true,
                "tags": ["developer", "rust"],
                "profile": {
                    "bio": "Software developer"
                }
            }
        });

        assert_eq!(Dot::get_string(&data, "user.name"), Some("John".to_string()));
        assert_eq!(Dot::get_number(&data, "user.age"), Some(30.0));
        assert_eq!(Dot::get_bool(&data, "user.active"), Some(true));
        assert!(Dot::get_array(&data, "user.tags").is_some());
        assert!(Dot::get_object(&data, "user.profile").is_some());

        assert_eq!(Dot::get_string(&data, "user.nonexistent"), None);
    }

    #[test]
    fn test_hashmap_access() {
        let mut map = HashMap::new();
        map.insert("simple".to_string(), "value".to_string());
        
        assert_eq!(Dot::get_from_map(&map, "simple"), Some("value".to_string()));
        assert_eq!(Dot::get_from_map(&map, "nonexistent"), None);
    }
}