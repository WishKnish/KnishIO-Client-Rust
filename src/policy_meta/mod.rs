//! PolicyMeta module for the KnishIO SDK
//!
//! This module provides the PolicyMeta struct and associated methods for managing
//! access control policies for metadata, ensuring exact compatibility with the
//! JavaScript PolicyMeta.js implementation.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::error::{KnishIOError, Result};

/// Represents access control policies for metadata
///
/// PolicyMeta manages read and write permissions for metadata keys,
/// providing default policy generation and normalization functionality.
/// This struct maintains exact compatibility with the JavaScript PolicyMeta class.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyMeta {
    /// The policy structure with read/write permissions
    pub policy: HashMap<String, HashMap<String, Vec<String>>>,
}

impl PolicyMeta {
    /// Create a new PolicyMeta instance
    ///
    /// Equivalent to new PolicyMeta(policy, metaKeys) in JavaScript SDK
    ///
    /// # Arguments
    ///
    /// * `policy` - Initial policy structure
    /// * `meta_keys` - Metadata keys for default policy generation
    ///
    /// # Returns
    ///
    /// New PolicyMeta instance with normalized and filled policies
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::policy_meta::PolicyMeta;
    /// use serde_json::json;
    ///
    /// let policy = json!({
    ///     "read": {
    ///         "pubkey": ["all"]
    ///     },
    ///     "write": {
    ///         "characters": ["all"]
    ///     }
    /// });
    ///
    /// let meta_keys = vec!["pubkey".to_string(), "characters".to_string()];
    /// let policy_meta = PolicyMeta::new(policy, meta_keys);
    /// ```
    pub fn new(policy: serde_json::Value, meta_keys: Vec<String>) -> Self {
        let mut policy_meta = PolicyMeta {
            policy: Self::normalize_policy(policy),
        };
        
        policy_meta.fill_default(meta_keys);
        policy_meta
    }

    /// Normalize policy structure
    ///
    /// Equivalent to PolicyMeta.normalizePolicy() in JavaScript SDK
    /// Filters policy structure to only include 'read' and 'write' keys
    ///
    /// # Arguments
    ///
    /// * `policy` - Raw policy data to normalize
    ///
    /// # Returns
    ///
    /// Normalized policy structure
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::policy_meta::PolicyMeta;
    /// use serde_json::json;
    ///
    /// let policy = json!({
    ///     "read": {
    ///         "pubkey": ["all"]
    ///     },
    ///     "write": {
    ///         "characters": ["all"]
    ///     },
    ///     "invalid": "this will be filtered out"
    /// });
    ///
    /// let normalized = PolicyMeta::normalize_policy(policy);
    /// assert!(normalized.contains_key("read"));
    /// assert!(normalized.contains_key("write"));
    /// assert!(!normalized.contains_key("invalid"));
    /// ```
    pub fn normalize_policy(policy: serde_json::Value) -> HashMap<String, HashMap<String, Vec<String>>> {
        let mut normalized = HashMap::new();

        if let Some(policy_obj) = policy.as_object() {
            for (action, value) in policy_obj {
                // Only process 'read' and 'write' actions
                if action == "read" || action == "write" {
                    if let Some(action_obj) = value.as_object() {
                        let mut action_map = HashMap::new();
                        
                        for (key, permissions) in action_obj {
                            if let Some(perms_array) = permissions.as_array() {
                                let perms: Vec<String> = perms_array
                                    .iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect();
                                action_map.insert(key.clone(), perms);
                            }
                        }
                        
                        normalized.insert(action.clone(), action_map);
                    }
                }
            }
        }

        normalized
    }

    /// Fill default policy values for metadata keys
    ///
    /// Equivalent to fillDefault(metaKeys) in JavaScript SDK
    /// Applies default read/write permissions for keys not explicitly defined
    ///
    /// # Arguments
    ///
    /// * `meta_keys` - List of metadata keys to ensure policies for
    ///
    /// # Default Rules
    ///
    /// - **Read permissions**: Default to `["all"]` for all keys
    /// - **Write permissions**: 
    ///   - `["all"]` for `characters` and `pubkey` keys
    ///   - `["self"]` for all other keys
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::policy_meta::PolicyMeta;
    /// use serde_json::json;
    ///
    /// let mut policy_meta = PolicyMeta {
    ///     policy: HashMap::new()
    /// };
    ///
    /// let meta_keys = vec!["pubkey".to_string(), "balance".to_string()];
    /// policy_meta.fill_default(meta_keys);
    ///
    /// // pubkey gets write: ["all"], balance gets write: ["self"]
    /// ```
    pub fn fill_default(&mut self, meta_keys: Vec<String>) {
        // Get existing policy keys for read and write
        let read_policy_keys: HashSet<String> = self.policy
            .get("read")
            .map(|read_map| read_map.keys().cloned().collect())
            .unwrap_or_else(HashSet::new);
            
        let write_policy_keys: HashSet<String> = self.policy
            .get("write")
            .map(|write_map| write_map.keys().cloned().collect())
            .unwrap_or_else(HashSet::new);

        // Ensure read and write policy maps exist
        if !self.policy.contains_key("read") {
            self.policy.insert("read".to_string(), HashMap::new());
        }
        if !self.policy.contains_key("write") {
            self.policy.insert("write".to_string(), HashMap::new());
        }

        // Calculate missing keys using diff operation
        let meta_keys_set: HashSet<String> = meta_keys.into_iter().collect();
        let read_missing: Vec<String> = Self::diff(&meta_keys_set, &read_policy_keys);
        let write_missing: Vec<String> = Self::diff(&meta_keys_set, &write_policy_keys);

        // Fill default read permissions (all keys get ["all"])
        if let Some(read_map) = self.policy.get_mut("read") {
            for key in read_missing {
                if !read_map.contains_key(&key) {
                    read_map.insert(key, vec!["all".to_string()]);
                }
            }
        }

        // Fill default write permissions (characters/pubkey get ["all"], others get ["self"])
        if let Some(write_map) = self.policy.get_mut("write") {
            for key in write_missing {
                if !write_map.contains_key(&key) {
                    let default_permission = if key == "characters" || key == "pubkey" {
                        vec!["all".to_string()]
                    } else {
                        vec!["self".to_string()]
                    };
                    write_map.insert(key, default_permission);
                }
            }
        }
    }

    /// Calculate the difference between two sets (equivalent to JavaScript diff function)
    ///
    /// # Arguments
    ///
    /// * `set_a` - First set
    /// * `set_b` - Second set to subtract from first
    ///
    /// # Returns
    ///
    /// Vector of elements in set_a but not in set_b
    fn diff(set_a: &HashSet<String>, set_b: &HashSet<String>) -> Vec<String> {
        set_a.difference(set_b).cloned().collect()
    }

    /// Get the policy structure
    ///
    /// Equivalent to get() in JavaScript SDK
    ///
    /// # Returns
    ///
    /// Reference to the policy HashMap
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::policy_meta::PolicyMeta;
    /// use serde_json::json;
    ///
    /// let policy = json!({
    ///     "read": {
    ///         "pubkey": ["all"]
    ///     }
    /// });
    ///
    /// let policy_meta = PolicyMeta::new(policy, vec![]);
    /// let policy_ref = policy_meta.get();
    /// ```
    pub fn get(&self) -> &HashMap<String, HashMap<String, Vec<String>>> {
        &self.policy
    }

    /// Convert policy to JSON string
    ///
    /// Equivalent to toJson() in JavaScript SDK
    ///
    /// # Returns
    ///
    /// Result containing JSON string or serialization error
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::policy_meta::PolicyMeta;
    /// use serde_json::json;
    ///
    /// let policy = json!({
    ///     "read": {
    ///         "pubkey": ["all"]
    ///     }
    /// });
    ///
    /// let policy_meta = PolicyMeta::new(policy, vec![]);
    /// let json_str = policy_meta.to_json().unwrap();
    /// ```
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.policy)
            .map_err(|e| KnishIOError::Serialization(e.to_string()))
    }

    /// Create PolicyMeta from GraphQL response data
    ///
    /// # Arguments
    ///
    /// * `data` - GraphQL response data containing policy information
    /// * `meta_keys` - Metadata keys for default policy generation
    ///
    /// # Returns
    ///
    /// Result containing new PolicyMeta instance or error
    pub fn create_from_graphql(data: &serde_json::Value, meta_keys: Vec<String>) -> Result<Self> {
        let policy = data.get("policy")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            
        Ok(Self::new(policy, meta_keys))
    }

    /// Create PolicyMeta from database array data
    ///
    /// # Arguments
    ///
    /// * `data` - Array-like data containing policy information
    /// * `meta_keys` - Metadata keys for default policy generation
    ///
    /// # Returns
    ///
    /// Result containing new PolicyMeta instance or error
    pub fn create_from_db(data: &serde_json::Value, meta_keys: Vec<String>) -> Result<Self> {
        let policy = if let Some(array) = data.as_array() {
            if array.len() > 0 {
                array[0].clone()
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            }
        } else {
            data.clone()
        };
        
        Ok(Self::new(policy, meta_keys))
    }

    /// Check if a specific permission is allowed
    ///
    /// # Arguments
    ///
    /// * `action` - The action to check ("read" or "write")
    /// * `key` - The metadata key
    /// * `bundle` - The bundle hash to check permission for
    ///
    /// # Returns
    ///
    /// True if the permission is allowed
    pub fn is_allowed(&self, action: &str, key: &str, bundle: &str) -> bool {
        if let Some(action_map) = self.policy.get(action) {
            if let Some(permissions) = action_map.get(key) {
                return permissions.contains(&"all".to_string()) || 
                       permissions.contains(&bundle.to_string()) ||
                       (permissions.contains(&"self".to_string()) && bundle == "self");
            }
        }
        false
    }

    /// Get permissions for a specific action and key
    ///
    /// # Arguments
    ///
    /// * `action` - The action ("read" or "write")
    /// * `key` - The metadata key
    ///
    /// # Returns
    ///
    /// Optional vector of permissions
    pub fn get_permissions(&self, action: &str, key: &str) -> Option<&Vec<String>> {
        self.policy.get(action)?.get(key)
    }

    /// Set permissions for a specific action and key
    ///
    /// # Arguments
    ///
    /// * `action` - The action ("read" or "write")
    /// * `key` - The metadata key
    /// * `permissions` - Vector of permission strings
    pub fn set_permissions(&mut self, action: &str, key: &str, permissions: Vec<String>) {
        self.policy
            .entry(action.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), permissions);
    }

    /// Remove permissions for a specific action and key
    ///
    /// # Arguments
    ///
    /// * `action` - The action ("read" or "write")
    /// * `key` - The metadata key
    ///
    /// # Returns
    ///
    /// Previously stored permissions, if any
    pub fn remove_permissions(&mut self, action: &str, key: &str) -> Option<Vec<String>> {
        self.policy.get_mut(action)?.remove(key)
    }

    /// Check if policy is empty
    ///
    /// # Returns
    ///
    /// True if no policies are defined
    pub fn is_empty(&self) -> bool {
        self.policy.is_empty() || 
        self.policy.values().all(|action_map| action_map.is_empty())
    }

    /// Clear all policies
    pub fn clear(&mut self) {
        self.policy.clear();
    }

    /// Get all metadata keys that have policies defined
    ///
    /// # Returns
    ///
    /// Set of all metadata keys with policies
    pub fn get_policy_keys(&self) -> HashSet<String> {
        let mut keys = HashSet::new();
        
        for action_map in self.policy.values() {
            for key in action_map.keys() {
                keys.insert(key.clone());
            }
        }
        
        keys
    }
}

impl Default for PolicyMeta {
    fn default() -> Self {
        Self {
            policy: HashMap::new(),
        }
    }
}

impl std::fmt::Display for PolicyMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_json() {
            Ok(json) => write!(f, "PolicyMeta({})", json),
            Err(_) => write!(f, "PolicyMeta(<invalid>)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_policy_meta_new() {
        let policy = json!({
            "read": {
                "pubkey": ["all"]
            },
            "write": {
                "characters": ["all"]
            }
        });

        let meta_keys = vec!["pubkey".to_string(), "characters".to_string(), "balance".to_string()];
        let policy_meta = PolicyMeta::new(policy, meta_keys);

        assert!(policy_meta.policy.contains_key("read"));
        assert!(policy_meta.policy.contains_key("write"));
        
        // Should have default policies for balance
        assert!(policy_meta.policy["read"].contains_key("balance"));
        assert!(policy_meta.policy["write"].contains_key("balance"));
    }

    #[test]
    fn test_normalize_policy() {
        let policy = json!({
            "read": {
                "pubkey": ["all"],
                "balance": ["self"]
            },
            "write": {
                "characters": ["all"]
            },
            "invalid": "this should be filtered out"
        });

        let normalized = PolicyMeta::normalize_policy(policy);

        assert_eq!(normalized.len(), 2);
        assert!(normalized.contains_key("read"));
        assert!(normalized.contains_key("write"));
        assert!(!normalized.contains_key("invalid"));

        assert_eq!(normalized["read"]["pubkey"], vec!["all"]);
        assert_eq!(normalized["read"]["balance"], vec!["self"]);
        assert_eq!(normalized["write"]["characters"], vec!["all"]);
    }

    #[test]
    fn test_fill_default() {
        let mut policy_meta = PolicyMeta {
            policy: HashMap::new(),
        };

        let meta_keys = vec![
            "pubkey".to_string(),
            "characters".to_string(),
            "balance".to_string(),
        ];

        policy_meta.fill_default(meta_keys);

        // Check read defaults (all keys should get ["all"])
        assert_eq!(policy_meta.policy["read"]["pubkey"], vec!["all"]);
        assert_eq!(policy_meta.policy["read"]["characters"], vec!["all"]);
        assert_eq!(policy_meta.policy["read"]["balance"], vec!["all"]);

        // Check write defaults
        assert_eq!(policy_meta.policy["write"]["pubkey"], vec!["all"]);      // Special case
        assert_eq!(policy_meta.policy["write"]["characters"], vec!["all"]);  // Special case
        assert_eq!(policy_meta.policy["write"]["balance"], vec!["self"]);    // Default case
    }

    #[test]
    fn test_fill_default_preserves_existing() {
        let mut policy_meta = PolicyMeta {
            policy: {
                let mut policy = HashMap::new();
                let mut read_map = HashMap::new();
                read_map.insert("balance".to_string(), vec!["bundle123".to_string()]);
                policy.insert("read".to_string(), read_map);
                policy
            },
        };

        let meta_keys = vec!["balance".to_string(), "pubkey".to_string()];
        policy_meta.fill_default(meta_keys);

        // Existing policy should be preserved
        assert_eq!(policy_meta.policy["read"]["balance"], vec!["bundle123"]);
        
        // New keys should get defaults
        assert_eq!(policy_meta.policy["read"]["pubkey"], vec!["all"]);
        assert_eq!(policy_meta.policy["write"]["pubkey"], vec!["all"]);
        assert_eq!(policy_meta.policy["write"]["balance"], vec!["self"]);
    }

    #[test]
    fn test_diff_function() {
        let set_a: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let set_b: HashSet<String> = ["b", "d"].iter().map(|s| s.to_string()).collect();

        let diff_result = PolicyMeta::diff(&set_a, &set_b);
        let mut sorted_diff = diff_result;
        sorted_diff.sort();

        assert_eq!(sorted_diff, vec!["a", "c"]);
    }

    #[test]
    fn test_get_and_to_json() {
        let policy = json!({
            "read": {
                "pubkey": ["all"]
            }
        });

        let policy_meta = PolicyMeta::new(policy, vec![]);
        let policy_ref = policy_meta.get();
        
        assert!(policy_ref.contains_key("read"));
        assert_eq!(policy_ref["read"]["pubkey"], vec!["all"]);

        let json_str = policy_meta.to_json().unwrap();
        assert!(json_str.contains("pubkey"));
        assert!(json_str.contains("all"));
    }

    #[test]
    fn test_create_from_graphql() {
        let data = json!({
            "policy": {
                "read": {
                    "pubkey": ["all"]
                }
            }
        });

        let meta_keys = vec!["pubkey".to_string()];
        let policy_meta = PolicyMeta::create_from_graphql(&data, meta_keys).unwrap();

        assert_eq!(policy_meta.policy["read"]["pubkey"], vec!["all"]);
    }

    #[test]
    fn test_create_from_db() {
        let data = json!([{
            "read": {
                "pubkey": ["all"]
            }
        }]);

        let meta_keys = vec!["pubkey".to_string()];
        let policy_meta = PolicyMeta::create_from_db(&data, meta_keys).unwrap();

        assert_eq!(policy_meta.policy["read"]["pubkey"], vec!["all"]);
    }

    #[test]
    fn test_is_allowed() {
        let policy = json!({
            "read": {
                "pubkey": ["all"],
                "balance": ["bundle123", "self"]
            },
            "write": {
                "private": ["self"]
            }
        });

        let policy_meta = PolicyMeta::new(policy, vec![]);

        // Test "all" permission
        assert!(policy_meta.is_allowed("read", "pubkey", "anyone"));
        
        // Test specific bundle permission
        assert!(policy_meta.is_allowed("read", "balance", "bundle123"));
        assert!(!policy_meta.is_allowed("read", "balance", "other_bundle"));
        
        // Test "self" permission
        assert!(policy_meta.is_allowed("read", "balance", "self"));
        assert!(policy_meta.is_allowed("write", "private", "self"));
        
        // Test non-existent key
        assert!(!policy_meta.is_allowed("read", "nonexistent", "anyone"));
    }

    #[test]
    fn test_permission_management() {
        let mut policy_meta = PolicyMeta::default();

        // Set permissions
        policy_meta.set_permissions("read", "test_key", vec!["all".to_string()]);
        assert_eq!(policy_meta.get_permissions("read", "test_key"), Some(&vec!["all".to_string()]));

        // Remove permissions
        let removed = policy_meta.remove_permissions("read", "test_key");
        assert_eq!(removed, Some(vec!["all".to_string()]));
        assert_eq!(policy_meta.get_permissions("read", "test_key"), None);
    }

    #[test]
    fn test_utility_methods() {
        let mut policy_meta = PolicyMeta::default();
        assert!(policy_meta.is_empty());

        policy_meta.set_permissions("read", "test", vec!["all".to_string()]);
        assert!(!policy_meta.is_empty());

        let keys = policy_meta.get_policy_keys();
        assert!(keys.contains("test"));

        policy_meta.clear();
        assert!(policy_meta.is_empty());
    }

    #[test]
    fn test_display() {
        let policy = json!({
            "read": {
                "pubkey": ["all"]
            }
        });

        let policy_meta = PolicyMeta::new(policy, vec![]);
        let display_str = format!("{}", policy_meta);
        assert!(display_str.starts_with("PolicyMeta("));
        assert!(display_str.contains("pubkey"));
    }

    #[test]
    fn test_javascript_compatibility() {
        // Test exact JavaScript SDK behavior reproduction

        // Test empty policy with meta keys (JavaScript line 87-105)
        let mut policy_meta = PolicyMeta::new(json!({}), vec![
            "pubkey".to_string(),
            "characters".to_string(),
            "balance".to_string(),
        ]);

        // Verify default policy generation matches JavaScript behavior
        assert_eq!(policy_meta.policy["read"]["pubkey"], vec!["all"]);
        assert_eq!(policy_meta.policy["read"]["characters"], vec!["all"]);
        assert_eq!(policy_meta.policy["read"]["balance"], vec!["all"]);

        assert_eq!(policy_meta.policy["write"]["pubkey"], vec!["all"]);      // Special case
        assert_eq!(policy_meta.policy["write"]["characters"], vec!["all"]);  // Special case
        assert_eq!(policy_meta.policy["write"]["balance"], vec!["self"]);    // Default case

        // Test normalization (JavaScript line 32-54)
        let raw_policy = json!({
            "read": {
                "pubkey": ["all"]
            },
            "write": {
                "balance": ["self"]
            },
            "invalid_action": {
                "some_key": ["all"]
            }
        });

        let normalized = PolicyMeta::normalize_policy(raw_policy);
        assert_eq!(normalized.len(), 2);
        assert!(normalized.contains_key("read"));
        assert!(normalized.contains_key("write"));
        assert!(!normalized.contains_key("invalid_action"));

        // Test toJson output format (JavaScript line 111)
        let json_output = policy_meta.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
        assert!(parsed.get("read").is_some());
        assert!(parsed.get("write").is_some());
    }
}