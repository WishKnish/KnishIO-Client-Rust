//! Array utility functions
//!
//! This module provides array manipulation functions that match the
//! JavaScript SDK's libraries/array.js functionality.

use std::collections::{HashMap, HashSet};
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Split a vector into chunks of specified size
///
/// Equivalent to chunkArray() in JavaScript
///
/// # Arguments
///
/// * `arr` - The vector to split into chunks
/// * `size` - The size of each chunk
///
/// # Returns
///
/// A vector of vectors, each containing at most `size` elements
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::chunk_array;
///
/// let data = vec![1, 2, 3, 4, 5, 6, 7];
/// let chunks = chunk_array(data, 3);
/// assert_eq!(chunks, vec![vec![1, 2, 3], vec![4, 5, 6], vec![7]]);
/// ```
pub fn chunk_array<T: Clone>(arr: Vec<T>, size: usize) -> Vec<Vec<T>> {
    if arr.is_empty() || size == 0 {
        return vec![];
    }

    let mut result = Vec::new();
    let mut i = 0;
    
    while i < arr.len() {
        let end = std::cmp::min(i + size, arr.len());
        result.push(arr[i..end].to_vec());
        i += size;
    }
    
    result
}

/// Deep clone a serde_json::Value with circular reference handling
///
/// Equivalent to deepCloning() in JavaScript
///
/// # Arguments
///
/// * `value` - The Value to deep clone
///
/// # Returns
///
/// A deep clone of the input value
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::deep_clone;
/// use serde_json::{json, Value};
///
/// let original = json!({"a": 1, "b": {"c": 2}});
/// let cloned = deep_clone(&original);
/// assert_eq!(original, cloned);
/// ```
pub fn deep_clone(value: &Value) -> Value {
    deep_clone_with_cache(value, &mut HashMap::new())
}

/// Internal function for deep cloning with circular reference detection
fn deep_clone_with_cache(value: &Value, cache: &mut HashMap<u64, Value>) -> Value {
    // Generate a hash for the value to detect circular references
    let mut hasher = DefaultHasher::new();
    
    // Hash the memory address as a proxy for object identity
    let value_ptr = value as *const Value as usize;
    value_ptr.hash(&mut hasher);
    let value_hash = hasher.finish();
    
    // Check if we've already cloned this value (circular reference)
    if let Some(cached) = cache.get(&value_hash) {
        return cached.clone();
    }
    
    match value {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(*b),
        Value::Number(n) => Value::Number(n.clone()),
        Value::String(s) => Value::String(s.clone()),
        Value::Array(arr) => {
            let cloned_array = Value::Array(vec![]);
            cache.insert(value_hash, cloned_array.clone());
            
            let mut result = Vec::new();
            for item in arr {
                result.push(deep_clone_with_cache(item, cache));
            }
            Value::Array(result)
        }
        Value::Object(obj) => {
            let cloned_object = Value::Object(serde_json::Map::new());
            cache.insert(value_hash, cloned_object.clone());
            
            let mut result = serde_json::Map::new();
            for (key, val) in obj {
                result.insert(key.clone(), deep_clone_with_cache(val, cache));
            }
            Value::Object(result)
        }
    }
}

/// Find the symmetric difference of multiple vectors
///
/// Equivalent to diff() in JavaScript - returns elements that are in one array but not in others
///
/// # Arguments
///
/// * `arrays` - Slice of vectors to compare
///
/// # Returns
///
/// A vector containing elements that appear in exactly one of the input arrays
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::diff;
///
/// let arr1 = vec![1, 2, 3];
/// let arr2 = vec![2, 3, 4];
/// let arr3 = vec![3, 4, 5];
/// let result = diff(&[arr1, arr2, arr3]);
/// assert_eq!(result, vec![1, 5]);
/// ```
pub fn diff<T: Clone + Eq + Hash>(arrays: &[Vec<T>]) -> Vec<T> {
    if arrays.is_empty() {
        return Vec::new();
    }
    
    let mut result = Vec::new();
    
    for (i, arr) in arrays.iter().enumerate() {
        // Collect all elements from other arrays
        let mut others_set = HashSet::new();
        for (j, other_arr) in arrays.iter().enumerate() {
            if i != j {
                for item in other_arr {
                    others_set.insert(item);
                }
            }
        }
        
        // Find elements in current array that are not in others
        for item in arr {
            if !others_set.contains(item) {
                result.push(item.clone());
            }
        }
    }
    
    result
}

/// Find the intersection of multiple vectors
///
/// Equivalent to intersect() in JavaScript - returns elements that are common to all arrays
///
/// # Arguments
///
/// * `arrays` - Slice of vectors to intersect
///
/// # Returns
///
/// A vector containing elements that appear in all input arrays
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::intersect;
///
/// let arr1 = vec![1, 2, 3, 4];
/// let arr2 = vec![2, 3, 4, 5];
/// let arr3 = vec![3, 4, 5, 6];
/// let result = intersect(&[arr1, arr2, arr3]);
/// assert_eq!(result, vec![3, 4]);
/// ```
pub fn intersect<T: Clone + Eq + Hash>(arrays: &[Vec<T>]) -> Vec<T> {
    if arrays.is_empty() {
        return Vec::new();
    }
    
    if arrays.len() == 1 {
        return arrays[0].clone();
    }
    
    let mut result = arrays[0].clone();
    
    for other_array in &arrays[1..] {
        let other_set: HashSet<&T> = other_array.iter().collect();
        result.retain(|item| other_set.contains(item));
    }
    
    result
}

/// Remove duplicates from a vector while preserving order
///
/// # Arguments
///
/// * `arr` - The vector to deduplicate
///
/// # Returns
///
/// A vector with duplicates removed, preserving first occurrence order
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::unique;
///
/// let data = vec![1, 2, 2, 3, 1, 4];
/// let result = unique(data);
/// assert_eq!(result, vec![1, 2, 3, 4]);
/// ```
pub fn unique<T: Clone + Eq + Hash>(arr: Vec<T>) -> Vec<T> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    
    for item in arr {
        if seen.insert(item.clone()) {
            result.push(item);
        }
    }
    
    result
}

/// Flatten a nested vector structure
///
/// # Arguments
///
/// * `arr` - The nested vector to flatten
///
/// # Returns
///
/// A flattened vector containing all elements from nested vectors
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::flatten;
///
/// let nested = vec![vec![1, 2], vec![3, 4], vec![5]];
/// let result = flatten(nested);
/// assert_eq!(result, vec![1, 2, 3, 4, 5]);
/// ```
pub fn flatten<T: Clone>(arr: Vec<Vec<T>>) -> Vec<T> {
    arr.into_iter().flatten().collect()
}

/// Check if all elements in a vector satisfy a predicate
///
/// # Arguments
///
/// * `arr` - The vector to check
/// * `predicate` - The predicate function to test each element
///
/// # Returns
///
/// `true` if all elements satisfy the predicate, `false` otherwise
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::all;
///
/// let data = vec![2, 4, 6, 8];
/// let result = all(&data, |x| x % 2 == 0);
/// assert!(result);
/// ```
pub fn all<T, F>(arr: &[T], predicate: F) -> bool
where
    F: Fn(&T) -> bool,
{
    arr.iter().all(predicate)
}

/// Check if any element in a vector satisfies a predicate
///
/// # Arguments
///
/// * `arr` - The vector to check
/// * `predicate` - The predicate function to test each element
///
/// # Returns
///
/// `true` if any element satisfies the predicate, `false` otherwise
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::any;
///
/// let data = vec![1, 3, 5, 8];
/// let result = any(&data, |x| x % 2 == 0);
/// assert!(result);
/// ```
pub fn any<T, F>(arr: &[T], predicate: F) -> bool
where
    F: Fn(&T) -> bool,
{
    arr.iter().any(predicate)
}

/// Group vector elements by a key function
///
/// # Arguments
///
/// * `arr` - The vector to group
/// * `key_fn` - Function that returns the grouping key for each element
///
/// # Returns
///
/// A HashMap where keys are the grouping keys and values are vectors of elements
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::group_by;
/// use std::collections::HashMap;
///
/// let data = vec!["apple", "banana", "apricot", "blueberry"];
/// let grouped = group_by(data, |s| s.chars().next().unwrap());
/// 
/// let mut expected = HashMap::new();
/// expected.insert('a', vec!["apple", "apricot"]);
/// expected.insert('b', vec!["banana", "blueberry"]);
/// assert_eq!(grouped, expected);
/// ```
pub fn group_by<T, K, F>(arr: Vec<T>, key_fn: F) -> HashMap<K, Vec<T>>
where
    K: Eq + Hash,
    F: Fn(&T) -> K,
{
    let mut groups = HashMap::new();
    
    for item in arr {
        let key = key_fn(&item);
        groups.entry(key).or_insert_with(Vec::new).push(item);
    }
    
    groups
}

/// Partition a vector into two vectors based on a predicate
///
/// # Arguments
///
/// * `arr` - The vector to partition
/// * `predicate` - The predicate function to test each element
///
/// # Returns
///
/// A tuple containing (elements that satisfy predicate, elements that don't)
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::array::partition;
///
/// let data = vec![1, 2, 3, 4, 5, 6];
/// let (evens, odds) = partition(data, |x| x % 2 == 0);
/// assert_eq!(evens, vec![2, 4, 6]);
/// assert_eq!(odds, vec![1, 3, 5]);
/// ```
pub fn partition<T, F>(arr: Vec<T>, predicate: F) -> (Vec<T>, Vec<T>)
where
    F: Fn(&T) -> bool,
{
    let mut true_vec = Vec::new();
    let mut false_vec = Vec::new();
    
    for item in arr {
        if predicate(&item) {
            true_vec.push(item);
        } else {
            false_vec.push(item);
        }
    }
    
    (true_vec, false_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_chunk_array() {
        let data = vec![1, 2, 3, 4, 5, 6, 7];
        let chunks = chunk_array(data, 3);
        assert_eq!(chunks, vec![vec![1, 2, 3], vec![4, 5, 6], vec![7]]);
        
        // Empty array
        let empty: Vec<i32> = vec![];
        let chunks = chunk_array(empty, 3);
        assert!(chunks.is_empty());
        
        // Zero size
        let data = vec![1, 2, 3];
        let chunks = chunk_array(data, 0);
        assert!(chunks.is_empty());
        
        // Single element chunks
        let data = vec![1, 2, 3];
        let chunks = chunk_array(data, 1);
        assert_eq!(chunks, vec![vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn test_deep_clone() {
        // Simple values
        let original = json!({"a": 1, "b": {"c": 2}});
        let cloned = deep_clone(&original);
        assert_eq!(original, cloned);
        
        // Array
        let original = json!([1, 2, {"a": 3}]);
        let cloned = deep_clone(&original);
        assert_eq!(original, cloned);
        
        // Primitives
        assert_eq!(deep_clone(&json!(null)), json!(null));
        assert_eq!(deep_clone(&json!(true)), json!(true));
        assert_eq!(deep_clone(&json!(42)), json!(42));
        assert_eq!(deep_clone(&json!("hello")), json!("hello"));
    }

    #[test]
    fn test_diff() {
        let arr1 = vec![1, 2, 3];
        let arr2 = vec![2, 3, 4];
        let arr3 = vec![3, 4, 5];
        let result = diff(&[arr1, arr2, arr3]);
        
        // Should contain elements that appear in only one array
        let mut result = result;
        result.sort();
        assert_eq!(result, vec![1, 5]);
        
        // Single array case
        let arr1 = vec![1, 2, 3];
        let result = diff(&[arr1]);
        assert_eq!(result, vec![1, 2, 3]);
        
        // Empty case
        let result: Vec<i32> = diff(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_intersect() {
        let arr1 = vec![1, 2, 3, 4];
        let arr2 = vec![2, 3, 4, 5];
        let arr3 = vec![3, 4, 5, 6];
        let result = intersect(&[arr1, arr2, arr3]);
        
        // Elements common to all arrays
        assert_eq!(result, vec![3, 4]);
        
        // Single array case
        let arr1 = vec![1, 2, 3];
        let result = intersect(&[arr1.clone()]);
        assert_eq!(result, arr1);
        
        // Empty case
        let result: Vec<i32> = intersect(&[]);
        assert!(result.is_empty());
        
        // No intersection
        let arr1 = vec![1, 2];
        let arr2 = vec![3, 4];
        let result = intersect(&[arr1, arr2]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_unique() {
        let data = vec![1, 2, 2, 3, 1, 4];
        let result = unique(data);
        assert_eq!(result, vec![1, 2, 3, 4]);
        
        // Already unique
        let data = vec![1, 2, 3];
        let result = unique(data);
        assert_eq!(result, vec![1, 2, 3]);
        
        // Empty
        let data: Vec<i32> = vec![];
        let result = unique(data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_flatten() {
        let nested = vec![vec![1, 2], vec![3, 4], vec![5]];
        let result = flatten(nested);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
        
        // Empty
        let nested: Vec<Vec<i32>> = vec![];
        let result = flatten(nested);
        assert!(result.is_empty());
        
        // Mixed sizes
        let nested = vec![vec![], vec![1], vec![2, 3]];
        let result = flatten(nested);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_all() {
        let data = vec![2, 4, 6, 8];
        assert!(all(&data, |x| x % 2 == 0));
        
        let data = vec![1, 2, 4, 6];
        assert!(!all(&data, |x| x % 2 == 0));
        
        // Empty array
        let data: Vec<i32> = vec![];
        assert!(all(&data, |x| x % 2 == 0)); // Vacuously true
    }

    #[test]
    fn test_any() {
        let data = vec![1, 3, 5, 8];
        assert!(any(&data, |x| x % 2 == 0));
        
        let data = vec![1, 3, 5];
        assert!(!any(&data, |x| x % 2 == 0));
        
        // Empty array
        let data: Vec<i32> = vec![];
        assert!(!any(&data, |x| x % 2 == 0));
    }

    #[test]
    fn test_group_by() {
        let data = vec!["apple", "banana", "apricot", "blueberry"];
        let grouped = group_by(data, |s| s.chars().next().unwrap());
        
        assert_eq!(grouped.get(&'a').unwrap(), &vec!["apple", "apricot"]);
        assert_eq!(grouped.get(&'b').unwrap(), &vec!["banana", "blueberry"]);
        
        // Empty array
        let data: Vec<&str> = vec![];
        let grouped = group_by(data, |s| s.chars().next().unwrap());
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_partition() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let (evens, odds) = partition(data, |x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4, 6]);
        assert_eq!(odds, vec![1, 3, 5]);
        
        // All true
        let data = vec![2, 4, 6];
        let (evens, odds) = partition(data, |x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4, 6]);
        assert!(odds.is_empty());
        
        // All false
        let data = vec![1, 3, 5];
        let (evens, odds) = partition(data, |x| x % 2 == 0);
        assert!(evens.is_empty());
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn test_js_compatibility() {
        // Test that our implementations match JavaScript behavior
        
        // chunkArray([1,2,3,4,5], 2) should return [[1,2], [3,4], [5]]
        let result = chunk_array(vec![1, 2, 3, 4, 5], 2);
        assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5]]);
        
        // diff([1,2,3], [2,3,4]) should return [1, 4]
        let arr1 = vec![1, 2, 3];
        let arr2 = vec![2, 3, 4];
        let mut result = diff(&[arr1, arr2]);
        result.sort();
        assert_eq!(result, vec![1, 4]);
        
        // intersect([1,2,3], [2,3,4]) should return [2, 3]
        let arr1 = vec![1, 2, 3];
        let arr2 = vec![2, 3, 4];
        let result = intersect(&[arr1, arr2]);
        assert_eq!(result, vec![2, 3]);
    }
}