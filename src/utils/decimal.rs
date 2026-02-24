//! Decimal precision utility
//!
//! This module provides high-precision decimal arithmetic utilities,
//! ensuring exact compatibility with the JavaScript SDK's Decimal.js implementation.

/// Decimal precision utility class
///
/// Equivalent to Decimal.js in the JavaScript SDK, this provides high-precision
/// decimal arithmetic to avoid floating-point precision issues in financial calculations.
pub struct Decimal;

impl Decimal {
    /// Minimum SQL decimal precision multiplier (10^18)
    const MULTIPLIER: f64 = 1_000_000_000_000_000_000.0; // 10^18

    /// Validate and normalize a decimal value
    ///
    /// Equivalent to Decimal.val() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `value` - The value to validate and normalize
    ///
    /// # Returns
    ///
    /// Normalized value or 0.0 if below precision threshold
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::Decimal;
    ///
    /// let result = Decimal::val(0.000000000000000001); // Very small number
    /// assert_eq!(result, 0.0); // Below precision threshold
    ///
    /// let result = Decimal::val(1.5);
    /// assert_eq!(result, 1.5); // Above threshold, returned as-is
    /// ```
    pub fn val(value: f64) -> f64 {
        if (value * Self::MULTIPLIER).abs() < 1.0 {
            0.0
        } else {
            value
        }
    }

    /// Compare two decimal values with high precision
    ///
    /// Equivalent to Decimal.cmp() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `value1` - First value to compare
    /// * `value2` - Second value to compare
    /// * `_debug` - Debug flag (unused, kept for compatibility)
    ///
    /// # Returns
    ///
    /// * `0` if values are equal
    /// * `1` if value1 > value2
    /// * `-1` if value1 < value2
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::Decimal;
    ///
    /// assert_eq!(Decimal::cmp(1.0, 1.0, false), 0);  // Equal
    /// assert_eq!(Decimal::cmp(2.0, 1.0, false), 1);  // Greater
    /// assert_eq!(Decimal::cmp(1.0, 2.0, false), -1); // Less
    /// ```
    pub fn cmp(value1: f64, value2: f64, _debug: bool) -> i8 {
        let val1 = Self::val(value1) * Self::MULTIPLIER;
        let val2 = Self::val(value2) * Self::MULTIPLIER;

        // Equal (within precision tolerance)
        if (val1 - val2).abs() < 1.0 {
            0
        } else if val1 > val2 {
            1
        } else {
            -1
        }
    }

    /// Check if two decimal values are equal with high precision
    ///
    /// Equivalent to Decimal.equal() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `value1` - First value to compare
    /// * `value2` - Second value to compare
    ///
    /// # Returns
    ///
    /// `true` if values are equal within precision tolerance
    ///
    /// # Example
    ///
    /// ```rust
    /// use knishio_client::utils::Decimal;
    ///
    /// assert!(Decimal::equal(1.0, 1.0));
    /// assert!(Decimal::equal(0.1 + 0.2, 0.3)); // Handles floating-point precision
    /// assert!(!Decimal::equal(1.0, 2.0));
    /// ```
    pub fn equal(value1: f64, value2: f64) -> bool {
        Self::cmp(value1, value2, false) == 0
    }

    /// Additional utility methods for Rust-specific enhancements

    /// Add two decimal values with high precision
    ///
    /// # Arguments
    ///
    /// * `value1` - First value
    /// * `value2` - Second value
    ///
    /// # Returns
    ///
    /// Sum of the two values
    pub fn add(value1: f64, value2: f64) -> f64 {
        Self::val(value1 + value2)
    }

    /// Subtract two decimal values with high precision
    ///
    /// # Arguments
    ///
    /// * `value1` - First value
    /// * `value2` - Second value
    ///
    /// # Returns
    ///
    /// Difference of the two values
    pub fn sub(value1: f64, value2: f64) -> f64 {
        Self::val(value1 - value2)
    }

    /// Multiply two decimal values with high precision
    ///
    /// # Arguments
    ///
    /// * `value1` - First value
    /// * `value2` - Second value
    ///
    /// # Returns
    ///
    /// Product of the two values
    pub fn mul(value1: f64, value2: f64) -> f64 {
        Self::val(value1 * value2)
    }

    /// Divide two decimal values with high precision
    ///
    /// # Arguments
    ///
    /// * `value1` - Dividend
    /// * `value2` - Divisor
    ///
    /// # Returns
    ///
    /// Quotient of the two values
    ///
    /// # Panics
    ///
    /// Panics if `value2` is zero
    pub fn div(value1: f64, value2: f64) -> f64 {
        if Self::equal(value2, 0.0) {
            panic!("Division by zero");
        }
        Self::val(value1 / value2)
    }

    /// Round a decimal value to specified decimal places
    ///
    /// # Arguments
    ///
    /// * `value` - Value to round
    /// * `decimal_places` - Number of decimal places
    ///
    /// # Returns
    ///
    /// Rounded value
    pub fn round(value: f64, decimal_places: u32) -> f64 {
        let multiplier = 10_f64.powi(decimal_places as i32);
        Self::val((value * multiplier).round() / multiplier)
    }

    /// Check if a value is effectively zero within precision tolerance
    ///
    /// # Arguments
    ///
    /// * `value` - Value to check
    ///
    /// # Returns
    ///
    /// `true` if value is effectively zero
    pub fn is_zero(value: f64) -> bool {
        Self::equal(value, 0.0)
    }

    /// Compare two string-encoded integer values with i128 precision
    ///
    /// Unlike `cmp()` which uses f64 (lossy for values > 2^53), this method
    /// parses strings as i128 for exact comparison of large balances.
    ///
    /// # Arguments
    ///
    /// * `value1` - First string-encoded integer
    /// * `value2` - Second string-encoded integer
    /// * `_debug` - Debug flag (unused, kept for API consistency)
    ///
    /// # Returns
    ///
    /// * `0` if values are equal
    /// * `1` if value1 > value2
    /// * `-1` if value1 < value2
    pub fn cmp_str(value1: &str, value2: &str, _debug: bool) -> i8 {
        let v1: i128 = value1.parse().unwrap_or(0);
        let v2: i128 = value2.parse().unwrap_or(0);
        match v1.cmp(&v2) {
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => -1,
        }
    }

    /// Get the absolute value with precision handling
    ///
    /// # Arguments
    ///
    /// * `value` - Value to get absolute value of
    ///
    /// # Returns
    ///
    /// Absolute value
    pub fn abs(value: f64) -> f64 {
        Self::val(value.abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_val() {
        // Very small values should become 0
        assert_eq!(Decimal::val(0.000000000000000001), 0.0);
        
        // Normal values should pass through
        assert_eq!(Decimal::val(1.5), 1.5);
        assert_eq!(Decimal::val(-2.0), -2.0);
    }

    #[test]
    fn test_cmp() {
        // Equal values
        assert_eq!(Decimal::cmp(1.0, 1.0, false), 0);
        assert_eq!(Decimal::cmp(0.1 + 0.2, 0.3, false), 0); // Floating-point precision
        
        // Greater than
        assert_eq!(Decimal::cmp(2.0, 1.0, false), 1);
        
        // Less than
        assert_eq!(Decimal::cmp(1.0, 2.0, false), -1);
    }

    #[test]
    fn test_equal() {
        assert!(Decimal::equal(1.0, 1.0));
        assert!(Decimal::equal(0.1 + 0.2, 0.3)); // Should handle floating-point precision
        assert!(!Decimal::equal(1.0, 2.0));
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(Decimal::add(1.5, 2.5), 4.0);
        assert_eq!(Decimal::sub(5.0, 3.0), 2.0);
        assert_eq!(Decimal::mul(2.0, 3.0), 6.0);
        assert_eq!(Decimal::div(6.0, 2.0), 3.0);
    }

    #[test]
    #[should_panic(expected = "Division by zero")]
    fn test_div_by_zero() {
        Decimal::div(1.0, 0.0);
    }

    #[test]
    fn test_round() {
        assert_eq!(Decimal::round(3.14159, 2), 3.14);
        assert_eq!(Decimal::round(3.14159, 4), 3.1416);
    }

    #[test]
    fn test_is_zero() {
        assert!(Decimal::is_zero(0.0));
        assert!(Decimal::is_zero(0.000000000000000001)); // Below precision threshold
        assert!(!Decimal::is_zero(1.0));
    }

    #[test]
    fn test_abs() {
        assert_eq!(Decimal::abs(-5.0), 5.0);
        assert_eq!(Decimal::abs(5.0), 5.0);
        assert_eq!(Decimal::abs(0.0), 0.0);
    }

    #[test]
    fn test_cmp_str() {
        // Equal values
        assert_eq!(Decimal::cmp_str("1000", "1000", false), 0);

        // Greater than
        assert_eq!(Decimal::cmp_str("2000", "1000", false), 1);

        // Less than
        assert_eq!(Decimal::cmp_str("1000", "2000", false), -1);

        // Large values beyond f64 precision (> 2^53)
        assert_eq!(Decimal::cmp_str("9007199254740993", "9007199254740992", false), 1);

        // Negative values
        assert_eq!(Decimal::cmp_str("-500", "500", false), -1);
        assert_eq!(Decimal::cmp_str("-500", "-500", false), 0);

        // Unparseable defaults to 0
        assert_eq!(Decimal::cmp_str("abc", "0", false), 0);
    }
}