//! Safe wrappers over the KnishIO-Crypto-Core 0.1.0 C ABI (`include/kcore.h`).
//!
//! Only the WOTS+ entry points the Rust SDK accelerates are bound: the ABI version, in-place
//! chain advancement and the WOTS+ address. Every kcore function returns 0 on success and -1 on
//! invalid arguments, leaving its outputs untouched on -1.

use std::os::raw::{c_char, c_int};

/// Characters in one WOTS+ chunk (64 bytes, hex encoded).
pub const CHUNK_HEX: usize = 128;
/// Most chains one `chains_hex` call accepts (kcore.h: n 1..64).
pub const MAX_CHAINS: usize = 64;
/// Characters in a WOTS+ private key (16 chunks).
pub const KEY_HEX: usize = 2048;
/// Characters in a WOTS+ address.
pub const ADDRESS_HEX: usize = 64;

/// Parallel lanes kcore uses when advancing chains (kcore.h: ways 1 or 4).
const WAYS: c_int = 4;

extern "C" {
    fn kcore_abi_version() -> c_int;
    fn kcore_chains_hex(chunks: *mut c_char, counts: *const c_int, n: usize, ways: c_int) -> c_int;
    fn kcore_wots_address(key_hex2048: *const c_char, address_hex64: *mut c_char) -> c_int;
}

/// The linked library's ABI version (`KCORE_ABI_VERSION`, 1 for kcore 0.1.0).
pub fn abi_version() -> i32 {
    // SAFETY: kcore_abi_version takes no arguments and only returns a constant.
    unsafe { kcore_abi_version() }
}

/// Advances `counts.len()` WOTS+ chains in place: chunk `i` (128 hex characters) becomes
/// hex(SHAKE256(chunk, 64 bytes)) applied `counts[i]` times.
///
/// Returns `false` when kcore rejects the arguments (a count outside 0..=64 or a non-hex chunk);
/// `chunks` is then unchanged.
///
/// # Panics
///
/// When `counts` holds 0 or more than 64 chains, or `chunks.len() != 128 * counts.len()`.
pub fn chains_hex(chunks: &mut [u8], counts: &[i32]) -> bool {
    assert!(
        (1..=MAX_CHAINS).contains(&counts.len()),
        "kcore chains_hex takes 1..=64 chains, got {}",
        counts.len()
    );
    assert_eq!(
        chunks.len(),
        CHUNK_HEX * counts.len(),
        "kcore chains_hex needs 128 hex characters per chain"
    );
    // SAFETY: `chunks` is a live, exclusively borrowed buffer of exactly 128 * n bytes and
    // `counts` a live buffer of n ints, with n = counts.len() in 1..=64 (asserted above).
    // kcore reads and writes only within those bounds (kcore.h:50-52) and keeps no pointer.
    let rc = unsafe {
        kcore_chains_hex(
            chunks.as_mut_ptr().cast::<c_char>(),
            counts.as_ptr(),
            counts.len(),
            WAYS,
        )
    };
    rc == 0
}

/// WOTS+ address (64 hex characters) of a 2048-hex-character private key.
///
/// Returns `None` when `key_hex` is not 2048 bytes long or kcore rejects it.
pub fn wots_address(key_hex: &[u8]) -> Option<[u8; ADDRESS_HEX]> {
    if key_hex.len() != KEY_HEX {
        return None;
    }
    let mut address = [0u8; ADDRESS_HEX];
    // SAFETY: `key_hex` is a live buffer of exactly 2048 bytes and `address` a live, exclusively
    // borrowed buffer of 64 bytes, the sizes kcore.h:54-55 reads and writes. No NUL is read or
    // written and kcore keeps no pointer.
    let rc = unsafe {
        kcore_wots_address(
            key_hex.as_ptr().cast::<c_char>(),
            address.as_mut_ptr().cast::<c_char>(),
        )
    };
    (rc == 0).then_some(address)
}
