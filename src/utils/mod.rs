//! Utility modules for the KnishIO SDK
//!
//! This module provides various utility functions used throughout the SDK.

pub mod strings;
pub mod decimal;
pub mod dot;
pub mod hex;
pub mod array;

// Re-export commonly used utilities
pub use strings::{
    base64_to_hex,
    hex_to_base64,
    chunk_substr,
    random_string,
    is_hex,
    charset_base_convert,
    buffer_to_hex_string,
    hex_string_to_buffer,
    to_camel_case,
    to_snake_case,
    is_numeric,
    trim_string,
};

pub use decimal::Decimal;
pub use dot::Dot;
pub use hex::{Hex, HexOptions};
pub use array::{
    chunk_array,
    deep_clone,
    diff,
    intersect,
    unique,
    flatten,
    all,
    any,
    group_by,
    partition,
};