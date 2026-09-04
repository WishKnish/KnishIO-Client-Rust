//! Memory hygiene and zeroization utilities for sensitive cryptographic material

use zeroize::Zeroize;

/// Overwrite byte slice contents with zero bytes
pub fn zeroize_bytes(buf: &mut [u8]) {
    buf.zeroize();
}

/// Execute a closure with a sensitive byte vector and guarantee zeroization upon completion
pub fn with_secure_bytes<T, F>(mut bytes: Vec<u8>, f: F) -> T
where
    F: FnOnce(&[u8]) -> T,
{
    struct Guard<'a>(&'a mut Vec<u8>);
    impl<'a> Drop for Guard<'a> {
        fn drop(&mut self) {
            self.0.zeroize();
        }
    }

    let guard = Guard(&mut bytes);
    f(guard.0)
}

/// Execute a closure with a sensitive String and guarantee zeroization upon completion
pub fn with_secure_string<T, F>(mut secret: String, f: F) -> T
where
    F: FnOnce(&str) -> T,
{
    struct StringGuard<'a>(&'a mut String);
    impl<'a> Drop for StringGuard<'a> {
        fn drop(&mut self) {
            self.0.zeroize();
        }
    }

    let guard = StringGuard(&mut secret);
    f(guard.0)
}

/// Constant-time comparison of two byte slices to prevent timing side-channel attacks
pub fn constant_time_equals(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}
