//! Utility modules for the KnishIO SDK
//!
//! This module provides various utility functions used throughout the SDK.

pub mod array;
pub mod decimal;
pub mod dot;
pub mod hex;
pub mod strings;

// Re-export commonly used utilities
pub use strings::{
    base64_to_hex, buffer_to_hex_string, charset_base_convert, chunk_substr, hex_string_to_buffer,
    hex_to_base64, is_hex, is_numeric, random_string, to_camel_case, to_snake_case, trim_string,
};

pub use array::{
    all, any, chunk_array, deep_clone, diff, flatten, group_by, intersect, partition, unique,
};
pub use decimal::Decimal;
pub use dot::Dot;
pub use hex::{Hex, HexOptions};
