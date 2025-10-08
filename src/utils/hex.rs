//! Hexadecimal utility functions
//!
//! This module provides utilities for converting between hexadecimal strings and byte arrays,
//! ensuring exact compatibility with the JavaScript SDK's Hex.js implementation.

/// Options for hexadecimal formatting
#[derive(Debug, Clone)]
pub struct HexOptions {
    /// Number of hex bytes grouped together with spaces between groups (0 = no grouping)
    pub grouping: usize,
    /// Number of groups per row (0 = no row splitting)
    pub rowlength: usize,
    /// Use uppercase hex characters
    pub uppercase: bool,
}

impl Default for HexOptions {
    fn default() -> Self {
        HexOptions {
            grouping: 0,
            rowlength: 0,
            uppercase: false,
        }
    }
}

impl HexOptions {
    /// Create new HexOptions with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set grouping option
    pub fn with_grouping(mut self, grouping: usize) -> Self {
        self.grouping = grouping;
        self
    }

    /// Set rowlength option
    pub fn with_rowlength(mut self, rowlength: usize) -> Self {
        self.rowlength = rowlength;
        self
    }

    /// Set uppercase option
    pub fn with_uppercase(mut self, uppercase: bool) -> Self {
        self.uppercase = uppercase;
        self
    }
}

/// Hexadecimal utility class
///
/// Equivalent to Hex.js in the JavaScript SDK, this provides utilities for converting
/// between hexadecimal strings and byte arrays with various formatting options.
pub struct Hex;

impl Hex {
    /// Convert byte array to hexadecimal string
    ///
    /// Equivalent to Hex.toHex() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `arr` - Byte array to convert
    /// * `options` - Optional formatting options
    ///
    /// # Returns
    ///
    /// Hexadecimal string representation of the byte array
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::{Hex, HexOptions};
    ///
    /// let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
    /// let hex = Hex::to_hex(&data, None);
    /// assert_eq!(hex, "48656c6c6f");
    ///
    /// // With formatting options
    /// let options = HexOptions::new()
    ///     .with_grouping(2)
    ///     .with_uppercase(true);
    /// let formatted = Hex::to_hex(&data, Some(options));
    /// assert_eq!(formatted, "48 65 6C 6C 6F");
    /// ```
    pub fn to_hex(arr: &[u8], options: Option<HexOptions>) -> String {
        let opts = options.unwrap_or_default();
        
        let hex_chars = if opts.uppercase {
            ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F']
        } else {
            ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f']
        };

        let mut result = String::new();
        let mut group = 0;
        let mut column = 0;

        for (i, &byte) in arr.iter().enumerate() {
            // Convert byte to hex
            let high = (byte >> 4) as usize;
            let low = (byte & 0x0F) as usize;
            result.push(hex_chars[high]);
            result.push(hex_chars[low]);

            // Skip formatting for last byte
            if i == arr.len() - 1 {
                break;
            }

            // Apply grouping if specified
            if opts.grouping > 0 {
                group += 1;
                if group == opts.grouping {
                    group = 0;

                    if opts.rowlength > 0 {
                        column += 1;
                        if column == opts.rowlength {
                            column = 0;
                            result.push('\n');
                        } else {
                            result.push(' ');
                        }
                    } else {
                        result.push(' ');
                    }
                }
            }
        }

        result
    }

    /// Convert hexadecimal string to byte array
    ///
    /// Equivalent to Hex.toUint8Array() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `hex_str` - Hexadecimal string to convert (whitespace is ignored)
    ///
    /// # Returns
    ///
    /// Byte vector representation of the hexadecimal string
    ///
    /// # Errors
    ///
    /// Returns error for invalid hexadecimal characters
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::Hex;
    ///
    /// let hex = "48656c6c6f";
    /// let bytes = Hex::to_uint8_array(hex).unwrap();
    /// assert_eq!(bytes, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    ///
    /// // Handles odd-length strings by prepending '0'
    /// let odd_hex = "FFF";
    /// let bytes = Hex::to_uint8_array(odd_hex).unwrap();
    /// assert_eq!(bytes, vec![0x0F, 0xFF]);
    ///
    /// // Ignores whitespace
    /// let spaced_hex = "48 65 6c 6c 6f";
    /// let bytes = Hex::to_uint8_array(spaced_hex).unwrap();
    /// assert_eq!(bytes, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    /// ```
    pub fn to_uint8_array(hex_str: &str) -> Result<Vec<u8>, String> {
        // Remove whitespace and convert to lowercase
        let mut target = hex_str.chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_lowercase())
            .collect::<String>();

        // Handle odd-length strings by prepending '0'
        if target.len() % 2 == 1 {
            target = format!("0{}", target);
        }

        let mut buffer = Vec::with_capacity(target.len() / 2);
        let hex_chars = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'];
        
        let chars: Vec<char> = target.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let high_char = chars[i];
            let low_char = chars[i + 1];

            // Find hex values
            let high_val = hex_chars.iter().position(|&c| c == high_char)
                .ok_or_else(|| format!("Unexpected character: '{}'", high_char))?;
            let low_val = hex_chars.iter().position(|&c| c == low_char)
                .ok_or_else(|| format!("Unexpected character: '{}'", low_char))?;

            buffer.push((high_val * 16 + low_val) as u8);
            i += 2;
        }

        Ok(buffer)
    }

    /// Convert byte array to hexadecimal string (simple version)
    ///
    /// Convenience method for basic hex conversion without formatting options
    ///
    /// # Arguments
    ///
    /// * `arr` - Byte array to convert
    ///
    /// # Returns
    ///
    /// Lowercase hexadecimal string
    pub fn encode(arr: &[u8]) -> String {
        Self::to_hex(arr, None)
    }

    /// Convert hexadecimal string to byte array (simple version)
    ///
    /// Convenience method for basic hex decoding
    ///
    /// # Arguments
    ///
    /// * `hex_str` - Hexadecimal string to convert
    ///
    /// # Returns
    ///
    /// Byte vector or error for invalid input
    pub fn decode(hex_str: &str) -> Result<Vec<u8>, String> {
        Self::to_uint8_array(hex_str)
    }

    /// Check if a string is valid hexadecimal
    ///
    /// # Arguments
    ///
    /// * `s` - String to validate
    ///
    /// # Returns
    ///
    /// `true` if the string contains only valid hex characters (ignoring whitespace)
    pub fn is_valid_hex(s: &str) -> bool {
        let cleaned = s.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();

        if cleaned.is_empty() {
            return false;
        }

        cleaned.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Convert hex string to uppercase
    ///
    /// # Arguments
    ///
    /// * `hex_str` - Hexadecimal string
    ///
    /// # Returns
    ///
    /// Uppercase hexadecimal string
    pub fn to_uppercase(hex_str: &str) -> String {
        hex_str.to_uppercase()
    }

    /// Convert hex string to lowercase
    ///
    /// # Arguments
    ///
    /// * `hex_str` - Hexadecimal string
    ///
    /// # Returns
    ///
    /// Lowercase hexadecimal string
    pub fn to_lowercase(hex_str: &str) -> String {
        hex_str.to_lowercase()
    }

    /// Get the byte length of a hex string
    ///
    /// # Arguments
    ///
    /// * `hex_str` - Hexadecimal string
    ///
    /// # Returns
    ///
    /// Number of bytes the hex string represents
    pub fn byte_length(hex_str: &str) -> usize {
        let cleaned = hex_str.chars()
            .filter(|c| !c.is_whitespace())
            .count();
        
        (cleaned + 1) / 2 // Round up for odd-length strings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hex_basic() {
        let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        let hex = Hex::to_hex(&data, None);
        assert_eq!(hex, "48656c6c6f");
    }

    #[test]
    fn test_to_hex_uppercase() {
        let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let options = HexOptions::new().with_uppercase(true);
        let hex = Hex::to_hex(&data, Some(options));
        assert_eq!(hex, "48656C6C6F");
    }

    #[test]
    fn test_to_hex_with_grouping() {
        let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let options = HexOptions::new().with_grouping(2);
        let hex = Hex::to_hex(&data, Some(options));
        assert_eq!(hex, "48 65 6c 6c 6f");
    }

    #[test]
    fn test_to_hex_with_rows() {
        let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x57, 0x6f];
        let options = HexOptions::new()
            .with_grouping(2)
            .with_rowlength(2);
        let hex = Hex::to_hex(&data, Some(options));
        assert_eq!(hex, "48 65\n6c 6c\n6f 20\n57 6f");
    }

    #[test]
    fn test_to_uint8_array_basic() {
        let hex = "48656c6c6f";
        let bytes = Hex::to_uint8_array(hex).unwrap();
        assert_eq!(bytes, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn test_to_uint8_array_odd_length() {
        let hex = "FFF";
        let bytes = Hex::to_uint8_array(hex).unwrap();
        assert_eq!(bytes, vec![0x0F, 0xFF]);
    }

    #[test]
    fn test_to_uint8_array_with_whitespace() {
        let hex = "48 65 6c 6c 6f";
        let bytes = Hex::to_uint8_array(hex).unwrap();
        assert_eq!(bytes, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn test_to_uint8_array_mixed_case() {
        let hex = "48656C6C6F";
        let bytes = Hex::to_uint8_array(hex).unwrap();
        assert_eq!(bytes, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn test_to_uint8_array_invalid_char() {
        let hex = "48G5";
        let result = Hex::to_uint8_array(hex);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected character"));
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = vec![0x00, 0x01, 0x02, 0xFE, 0xFF];
        let encoded = Hex::encode(&original);
        let decoded = Hex::decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_is_valid_hex() {
        assert!(Hex::is_valid_hex("48656c6c6f"));
        assert!(Hex::is_valid_hex("48 65 6c 6c 6f"));
        assert!(Hex::is_valid_hex("ABCDEF"));
        assert!(Hex::is_valid_hex("0123456789abcdef"));
        
        assert!(!Hex::is_valid_hex(""));
        assert!(!Hex::is_valid_hex("G"));
        assert!(!Hex::is_valid_hex("48G5"));
        assert!(!Hex::is_valid_hex("xyz"));
    }

    #[test]
    fn test_case_conversion() {
        let hex = "48656c6c6f";
        assert_eq!(Hex::to_uppercase(hex), "48656C6C6F");
        assert_eq!(Hex::to_lowercase("48656C6C6F"), "48656c6c6f");
    }

    #[test]
    fn test_byte_length() {
        assert_eq!(Hex::byte_length("48656c6c6f"), 5);
        assert_eq!(Hex::byte_length("48 65 6c 6c 6f"), 5);
        assert_eq!(Hex::byte_length("FFF"), 2); // Odd length rounds up
        assert_eq!(Hex::byte_length(""), 0);
    }

    #[test]
    fn test_empty_array() {
        let data: Vec<u8> = vec![];
        let hex = Hex::to_hex(&data, None);
        assert_eq!(hex, "");
        
        let bytes = Hex::to_uint8_array("").unwrap();
        assert_eq!(bytes, vec![]);
    }

    #[test]
    fn test_single_byte() {
        let data = vec![0x42];
        let hex = Hex::to_hex(&data, None);
        assert_eq!(hex, "42");
        
        let bytes = Hex::to_uint8_array("42").unwrap();
        assert_eq!(bytes, vec![0x42]);
    }

    #[test]
    fn test_compatibility_with_js_examples() {
        // Test cases that should match JavaScript SDK behavior exactly
        
        // Basic conversion
        let data = vec![255, 254, 253];
        let hex = Hex::to_hex(&data, None);
        assert_eq!(hex, "fffefd");
        
        // Uppercase
        let options = HexOptions::new().with_uppercase(true);
        let hex = Hex::to_hex(&data, Some(options));
        assert_eq!(hex, "FFFEFD");
        
        // Grouping
        let options = HexOptions::new().with_grouping(1);
        let hex = Hex::to_hex(&data, Some(options));
        assert_eq!(hex, "ff fe fd");
        
        // Reverse conversion
        let bytes = Hex::to_uint8_array("fffefd").unwrap();
        assert_eq!(bytes, vec![255, 254, 253]);
    }
}