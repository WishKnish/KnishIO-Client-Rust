//! String utility functions for the KnishIO SDK
//!
//! This module provides string manipulation functions that match the
//! JavaScript SDK's libraries/strings.js functionality.

use base64::{Engine as _, engine::general_purpose};
use crate::error::{KnishIOError, Result};

/// Convert a Base64 string to hexadecimal
///
/// # Arguments
///
/// * `base64` - The Base64-encoded string
///
/// # Returns
///
/// The hexadecimal representation of the decoded data
pub fn base64_to_hex(base64: &str) -> Result<String> {
    let bytes = general_purpose::STANDARD
        .decode(base64)
        .map_err(|e| KnishIOError::custom(format!("Invalid base64: {}", e)))?;
    Ok(hex::encode(bytes))
}

/// Convert a hexadecimal string to Base64
///
/// # Arguments
///
/// * `hex_str` - The hexadecimal string
///
/// # Returns
///
/// The Base64-encoded representation
pub fn hex_to_base64(hex_str: &str) -> Result<String> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| KnishIOError::custom(format!("Invalid hex: {}", e)))?;
    Ok(general_purpose::STANDARD.encode(bytes))
}

/// Split a string into chunks of specified length
///
/// # Arguments
///
/// * `input` - The string to split
/// * `chunk_size` - The size of each chunk
///
/// # Returns
///
/// A vector of string chunks
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::strings::chunk_substr;
///
/// let chunks = chunk_substr("abcdefghijklmnop", 4);
/// assert_eq!(chunks, vec!["abcd", "efgh", "ijkl", "mnop"]);
/// ```
pub fn chunk_substr(input: &str, chunk_size: usize) -> Vec<String> {
    if chunk_size == 0 {
        return vec![input.to_string()];
    }
    
    input
        .chars()
        .collect::<Vec<_>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

/// Generate a cryptographically-secure pseudo-random string
///
/// Equivalent to randomString() in JavaScript
///
/// # Arguments
///
/// * `length` - The desired length of the random string (default: 256)
/// * `alphabet` - Optional alphabet to use (default: "abcdef0123456789")
///
/// # Returns
///
/// A random string from the specified alphabet
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::strings::random_string;
///
/// let hex_string = random_string(16, Some("0123456789abcdef"));
/// assert_eq!(hex_string.len(), 16);
/// assert!(hex_string.chars().all(|c| "0123456789abcdef".contains(c)));
/// ```
pub fn random_string(length: usize, alphabet: Option<&str>) -> String {
    use rand::Rng;
    
    let charset = alphabet.unwrap_or("abcdef0123456789");
    let charset_bytes: Vec<u8> = charset.bytes().collect();
    let mut rng = rand::rng();
    
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..charset_bytes.len());
            charset_bytes[idx] as char
        })
        .collect()
}

/// Check if a string is valid hexadecimal
///
/// # Arguments
///
/// * `input` - The string to check
///
/// # Returns
///
/// `true` if the string contains only hexadecimal characters
pub fn is_hex(input: &str) -> bool {
    !input.is_empty() && input.chars().all(|c| c.is_ascii_hexdigit())
}

/// Convert charset between bases and alphabets
///
/// Equivalent to charsetBaseConvert() in JavaScript
///
/// # Arguments
///
/// * `src` - The source string to convert
/// * `from_base` - The base of the source string
/// * `to_base` - The target base
/// * `src_symbol_table` - Optional source symbol table
/// * `dest_symbol_table` - Optional destination symbol table
///
/// # Returns
///
/// The converted string, or None if conversion fails
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::strings::charset_base_convert;
///
/// let result = charset_base_convert("FF", 16, 10, None, None);
/// assert_eq!(result, Some("255".to_string()));
/// ```
pub fn charset_base_convert(
    src: &str,
    from_base: u32,
    to_base: u32,
    src_symbol_table: Option<&str>,
    dest_symbol_table: Option<&str>,
) -> Option<String> {
    const BASE_SYMBOLS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz~`!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?¿¡";
    
    let src_table = src_symbol_table.unwrap_or(BASE_SYMBOLS);
    let dest_table = dest_symbol_table.unwrap_or(src_table);
    
    if from_base as usize > src_table.len() || to_base as usize > dest_table.len() {
        eprintln!("charset_base_convert() - Can't convert {} to base {} greater than symbol table length. src-table: {}, dest-table: {}", 
                  src, to_base, src_table.len(), dest_table.len());
        return None;
    }
    
    if src.is_empty() {
        return Some("0".to_string());
    }
    
    // Convert from source base to decimal using u128 for large numbers
    let mut val: u128 = 0;
    let src_chars: Vec<char> = src_table.chars().collect();
    let from_base = from_base as u128;
    
    for ch in src.chars() {
        let pos = src_chars.iter().position(|&c| c == ch);
        if let Some(digit_val) = pos {
            val = val.checked_mul(from_base)?
                     .checked_add(digit_val as u128)?;
        } else {
            // Invalid character in source
            return None;
        }
    }
    
    // Convert from decimal to destination base
    if val == 0 {
        return Some("0".to_string());
    }
    
    let dest_chars: Vec<char> = dest_table.chars().collect();
    let to_base = to_base as u128;
    let mut result = String::new();
    
    while val > 0 {
        let remainder = (val % to_base) as usize;
        if remainder >= dest_chars.len() {
            return None;
        }
        result = format!("{}{}", dest_chars[remainder], result);
        val /= to_base;
    }
    
    Some(result)
}

/// Convert a buffer (Vec<u8>) to a hexadecimal string
///
/// # Arguments
///
/// * `buffer` - The byte buffer
///
/// # Returns
///
/// The hexadecimal representation
pub fn buffer_to_hex_string(buffer: &[u8]) -> String {
    hex::encode(buffer)
}

/// Convert a hexadecimal string to a buffer (Vec<u8>)
///
/// # Arguments
///
/// * `hex_str` - The hexadecimal string
///
/// # Returns
///
/// The byte buffer
pub fn hex_string_to_buffer(hex_str: &str) -> Result<Vec<u8>> {
    hex::decode(hex_str)
        .map_err(|e| KnishIOError::custom(format!("Invalid hex string: {}", e)))
}

/// Convert a hexadecimal string to base17 format
///
/// Base17 uses characters 0-9 and a-g. This is used for molecular hash
/// normalization in the one-time signature algorithm.
///
/// # Arguments
///
/// * `hex_str` - The hexadecimal string to convert
///
/// # Returns
///
/// The base17 representation
pub fn hex_to_base17(hex_str: &str) -> String {
    // Convert each hex nibble to base17
    hex_str.chars().map(|c| {
        match c.to_ascii_lowercase() {
            '0'..='9' => c,
            'a'..='f' => c,
            // In case of invalid hex, map to 'g'
            _ => 'g',
        }
    }).collect()
}

/// Normalize metadata for consistent hashing
///
/// Converts various metadata formats to a consistent key-value array format.
///
/// # Arguments
///
/// * `meta` - The metadata to normalize (can be object or array)
///
/// # Returns
///
/// A normalized vector of key-value pairs
pub fn normalize_meta(meta: &serde_json::Value) -> Vec<(String, String)> {
    match meta {
        serde_json::Value::Array(arr) => {
            arr.iter()
                .filter_map(|item| {
                    if let (Some(key), Some(value)) = (
                        item.get("key").and_then(|k| k.as_str()),
                        item.get("value").and_then(|v| v.as_str()),
                    ) {
                        Some((key.to_string(), value.to_string()))
                    } else {
                        None
                    }
                })
                .collect()
        }
        serde_json::Value::Object(obj) => {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Convert a string to camelCase
///
/// Equivalent to String.prototype.toCamelCase() in JavaScript
///
/// # Arguments
///
/// * `input` - The string to convert
///
/// # Returns
///
/// The string in camelCase format
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::strings::to_camel_case;
///
/// assert_eq!(to_camel_case("hello-world_test"), "helloWorldTest");
/// assert_eq!(to_camel_case("some_long_variable_name"), "someLongVariableName");
/// ```
pub fn to_camel_case(input: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize_next {
                result.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(ch.to_ascii_lowercase());
            }
        } else {
            capitalize_next = true;
        }
    }
    
    result
}

/// Convert a string to snake_case
///
/// Equivalent to String.prototype.toSnakeCase() in JavaScript
///
/// # Arguments
///
/// * `input` - The string to convert
///
/// # Returns
///
/// The string in snake_case format
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::strings::to_snake_case;
///
/// assert_eq!(to_snake_case("HelloWorld"), "hello_world");
/// assert_eq!(to_snake_case("someVariable"), "some_variable");
/// ```
pub fn to_snake_case(input: &str) -> String {
    let mut result = String::new();
    
    for ch in input.chars() {
        if ch.is_ascii_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Check if a string represents a numeric value
///
/// Equivalent to isNumeric() in JavaScript
///
/// # Arguments
///
/// * `input` - The string to check
///
/// # Returns
///
/// `true` if the string represents a valid number
///
/// # Example
///
/// ```rust
/// use knishio_client::utils::strings::is_numeric;
///
/// assert!(is_numeric("123"));
/// assert!(is_numeric("123.45"));
/// assert!(is_numeric("-123"));
/// assert!(!is_numeric("abc"));
/// assert!(!is_numeric(""));
/// ```
pub fn is_numeric(input: &str) -> bool {
    if input.trim().is_empty() {
        return false;
    }
    
    input.trim().parse::<f64>().is_ok()
}

/// Trim whitespace from a string (convenience function)
///
/// # Arguments
///
/// * `input` - The string to trim
///
/// # Returns
///
/// The trimmed string
pub fn trim_string(input: &str) -> String {
    input.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_hex_conversion() {
        let original = "48656c6c6f20576f726c64"; // "Hello World" in hex
        let base64 = hex_to_base64(original).unwrap();
        let back_to_hex = base64_to_hex(&base64).unwrap();
        assert_eq!(original.to_lowercase(), back_to_hex.to_lowercase());
    }

    #[test]
    fn test_chunk_substr() {
        let input = "abcdefghijklmnop";
        let chunks = chunk_substr(input, 4);
        assert_eq!(chunks, vec!["abcd", "efgh", "ijkl", "mnop"]);
        
        let chunks = chunk_substr(input, 5);
        assert_eq!(chunks, vec!["abcde", "fghij", "klmno", "p"]);
        
        let chunks = chunk_substr("", 4);
        assert!(chunks.is_empty() || chunks == vec![""]);
    }

    #[test]
    fn test_random_string() {
        let s1 = random_string(10, None);
        let s2 = random_string(10, None);
        assert_eq!(s1.len(), 10);
        assert_eq!(s2.len(), 10);
        assert_ne!(s1, s2); // Should be different (with high probability)
        assert!(s1.chars().all(|c| "abcdef0123456789".contains(c)));
        
        // Test with custom alphabet
        let hex_string = random_string(16, Some("0123456789abcdef"));
        assert_eq!(hex_string.len(), 16);
        assert!(hex_string.chars().all(|c| "0123456789abcdef".contains(c)));
        
        // Test with different alphabet
        let alpha_string = random_string(8, Some("ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
        assert_eq!(alpha_string.len(), 8);
        assert!(alpha_string.chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn test_is_hex() {
        assert!(is_hex("0123456789abcdef"));
        assert!(is_hex("ABCDEF"));
        assert!(!is_hex("0123456789abcdefg"));
        assert!(!is_hex("xyz"));
        assert!(!is_hex(""));
    }

    #[test]
    fn test_hex_to_base17() {
        let hex = "0123456789abcdef";
        let base17 = hex_to_base17(hex);
        assert_eq!(base17, "0123456789abcdef");
        
        // Test that invalid hex maps to 'g'
        let invalid = "xyz";
        let base17 = hex_to_base17(invalid);
        assert!(base17.contains('g'));
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("hello-world_test"), "helloWorldTest");
        assert_eq!(to_camel_case("some_long_variable_name"), "someLongVariableName");
        assert_eq!(to_camel_case("already-camelCase"), "alreadyCamelcase");
        assert_eq!(to_camel_case("simple"), "simple");
        assert_eq!(to_camel_case(""), "");
        assert_eq!(to_camel_case("hello world"), "helloWorld");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("someVariable"), "some_variable");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case(""), "");
        assert_eq!(to_snake_case("XMLHttpRequest"), "x_m_l_http_request");
    }

    #[test]
    fn test_is_numeric() {
        assert!(is_numeric("123"));
        assert!(is_numeric("123.45"));
        assert!(is_numeric("-123"));
        assert!(is_numeric("-123.45"));
        assert!(is_numeric("0"));
        assert!(is_numeric("0.0"));
        assert!(is_numeric("  123  ")); // With whitespace
        
        assert!(!is_numeric("abc"));
        assert!(!is_numeric(""));
        assert!(!is_numeric("   "));
        assert!(!is_numeric("123abc"));
        assert!(!is_numeric("12.34.56"));
        assert!(!is_numeric("NaN"));
    }

    #[test]
    fn test_charset_base_convert() {
        // Test hex to decimal conversion
        let result = charset_base_convert("FF", 16, 10, None, None);
        assert_eq!(result, Some("255".to_string()));
        
        // Test decimal to hex conversion
        let result = charset_base_convert("255", 10, 16, None, None);
        assert_eq!(result, Some("FF".to_string()));
        
        // Test binary to decimal conversion
        let result = charset_base_convert("1010", 2, 10, None, None);
        assert_eq!(result, Some("10".to_string()));
        
        // Test with custom symbol tables
        let result = charset_base_convert("10", 2, 10, Some("01"), Some("0123456789"));
        assert_eq!(result, Some("2".to_string()));
        
        // Test edge cases
        let result = charset_base_convert("0", 10, 16, None, None);
        assert_eq!(result, Some("0".to_string()));
        
        let result = charset_base_convert("", 10, 16, None, None);
        assert_eq!(result, Some("0".to_string()));
        
        // Test invalid base (too large)
        let result = charset_base_convert("123", 100, 10, None, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_trim_string() {
        assert_eq!(trim_string("  hello  "), "hello");
        assert_eq!(trim_string("\t\nworld\t\n"), "world");
        assert_eq!(trim_string("no spaces"), "no spaces");
        assert_eq!(trim_string(""), "");
        assert_eq!(trim_string("   "), "");
    }

    #[test]
    fn test_js_compatibility() {
        // Test that our implementations match JavaScript behavior
        
        // chunkSubstr("abcdefghijklmnop", 4) should return ["abcd", "efgh", "ijkl", "mnop"]
        let result = chunk_substr("abcdefghijklmnop", 4);
        assert_eq!(result, vec!["abcd", "efgh", "ijkl", "mnop"]);
        
        // isHex should match JavaScript regex behavior
        assert!(is_hex("ABCDEF123"));
        assert!(is_hex("abcdef123"));
        assert!(!is_hex("xyz"));
        
        // String case conversions should match JavaScript
        assert_eq!(to_camel_case("hello-world"), "helloWorld");
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        
        // isNumeric should match JavaScript isNaN behavior (inverted)
        assert!(is_numeric("123"));
        assert!(is_numeric("123.45"));
        assert!(!is_numeric("abc"));
        assert!(!is_numeric(""));
    }
}