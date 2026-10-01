//! Optional KnishIO-Crypto-Core backend for the WOTS+ hot loops.
//!
//! With the `kcore` feature, WOTS+ chain advancement (signing, verification, OTS fragments) and
//! the WOTS+ address run in the statically linked kcore C library. Every entry point here returns
//! `None` whenever kcore cannot take the input exactly as the SDK's own loop would, and the caller
//! then runs that loop unchanged; without the feature every entry point returns `None`.

/// Characters in one WOTS+ chunk (64 bytes, hex encoded).
#[cfg(feature = "kcore")]
const CHUNK_HEX: usize = 128;
/// Most chains one kcore call takes, and the most iterations per chain.
const MAX_CHAINS: usize = 64;
#[cfg(feature = "kcore")]
const MAX_COUNT: usize = 64;
/// Characters in a WOTS+ private key.
#[cfg(feature = "kcore")]
const KEY_HEX: usize = 2048;

/// The linked kcore's ABI check, run once.
#[cfg(feature = "kcore")]
static AVAILABLE: std::sync::LazyLock<bool> =
    std::sync::LazyLock::new(|| knishio_kcore::abi_version() == 1);

/// Whether WOTS+ operations run through kcore: the `kcore` feature is compiled in and the linked
/// library reports ABI version 1.
#[cfg(feature = "kcore")]
pub fn available() -> bool {
    *AVAILABLE
}

/// Whether WOTS+ operations run through kcore; always `false` without the `kcore` feature.
#[cfg(not(feature = "kcore"))]
#[inline]
pub fn available() -> bool {
    false
}

#[cfg(feature = "kcore")]
fn is_lower_hex(bytes: &[u8]) -> bool {
    bytes.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Advances `counts.len()` WOTS+ chains: chunk `i` of `chunks_hex` (128 lowercase hex characters)
/// becomes `shake256(chunk, 512)` applied `counts[i]` times. Returns the concatenated chunks, or
/// `None` (use the SDK loop) unless kcore is available, there are 1..=64 chains, `chunks_hex` is
/// exactly `128 * n` lowercase hex characters and every count is at most 64.
#[cfg(feature = "kcore")]
pub(crate) fn chains_hex(chunks_hex: &str, counts: &[usize]) -> Option<String> {
    let n = counts.len();
    if !available()
        || !(1..=MAX_CHAINS).contains(&n)
        || chunks_hex.len() != CHUNK_HEX * n
        || !is_lower_hex(chunks_hex.as_bytes())
        || counts.iter().any(|&count| count > MAX_COUNT)
    {
        return None;
    }
    let mut kcore_counts = [0i32; MAX_CHAINS];
    for (slot, &count) in kcore_counts.iter_mut().zip(counts) {
        *slot = i32::try_from(count).ok()?;
    }
    let mut chunks = chunks_hex.as_bytes().to_vec();
    if !knishio_kcore::chains_hex(&mut chunks, &kcore_counts[..n]) {
        return None;
    }
    String::from_utf8(chunks).ok()
}

/// Without the `kcore` feature: always `None`.
#[cfg(not(feature = "kcore"))]
#[inline]
pub(crate) fn chains_hex(_chunks_hex: &str, _counts: &[usize]) -> Option<String> {
    None
}

/// [`chains_hex`] with the counts given as signed WOTS+ iteration numbers (`8 ∓ n`). Any count
/// outside `0..=64` or more than 64 counts gives `None`. The iterator is not consumed when kcore
/// is unavailable, so callers pay nothing for building the counts without the feature.
pub(crate) fn chains_hex_signed<I>(chunks_hex: &str, counts: I) -> Option<String>
where
    I: IntoIterator<Item = i32>,
{
    if !available() {
        return None;
    }
    let mut buf = [0usize; MAX_CHAINS];
    let mut n = 0;
    for count in counts {
        *buf.get_mut(n)? = usize::try_from(count).ok()?;
        n += 1;
    }
    chains_hex(chunks_hex, &buf[..n])
}

/// WOTS+ address of a 2048-character lowercase-hex private key, or `None` (use the SDK loop).
#[cfg(feature = "kcore")]
pub(crate) fn wots_address(key_hex: &str) -> Option<String> {
    if !available() || key_hex.len() != KEY_HEX || !is_lower_hex(key_hex.as_bytes()) {
        return None;
    }
    let address = knishio_kcore::wots_address(key_hex.as_bytes())?;
    String::from_utf8(address.to_vec()).ok()
}

/// Without the `kcore` feature: always `None`.
#[cfg(not(feature = "kcore"))]
#[inline]
pub(crate) fn wots_address(_key_hex: &str) -> Option<String> {
    None
}

#[cfg(all(test, feature = "kcore"))]
mod tests {
    use super::*;
    use crate::crypto::{shake256, wots_address_sdk};

    /// Deterministic lowercase hex of `bits / 4` characters.
    fn hex_of(label: &str, bits: usize) -> String {
        shake256(label, bits)
    }

    fn sdk_chains(chunks_hex: &str, counts: &[usize]) -> String {
        let mut out = String::with_capacity(chunks_hex.len());
        for (i, &count) in counts.iter().enumerate() {
            let mut chunk = chunks_hex[i * CHUNK_HEX..(i + 1) * CHUNK_HEX].to_string();
            for _ in 0..count {
                chunk = shake256(&chunk, 512);
            }
            out.push_str(&chunk);
        }
        out
    }

    #[test]
    fn kcore_is_available_with_the_feature() {
        assert!(available(), "kcore feature built but abi_version() != 1");
    }

    #[test]
    fn chains_hex_matches_the_sdk_loop() {
        for case in 0..20usize {
            let n = case % 16 + 1;
            let chunks: String = (0..n).map(|j| hex_of(&format!("kcore-chain-{case}-{j}"), 512)).collect();
            let counts: Vec<usize> = (0..n).map(|j| (case * 7 + j * 3) % 17).collect();
            let sdk = sdk_chains(&chunks, &counts);
            assert_eq!(chains_hex(&chunks, &counts), Some(sdk), "case {case}: n={n} counts={counts:?}");
        }
    }

    #[test]
    fn chains_hex_accepts_the_boundary_count_and_chain_number() {
        let chunks: String = (0..MAX_CHAINS).map(|j| hex_of(&format!("kcore-edge-{j}"), 512)).collect();
        let mut counts = vec![0usize; MAX_CHAINS];
        counts[0] = MAX_COUNT;
        assert_eq!(chains_hex(&chunks, &counts), Some(sdk_chains(&chunks, &counts)), "64 chains, count 64");
    }

    #[test]
    fn chains_hex_refuses_inputs_the_sdk_loop_must_handle() {
        let chunk = hex_of("kcore-refuse", 512);
        assert_eq!(chains_hex(&chunk[..127], &[1]), None, "127-character chunk");
        assert_eq!(chains_hex(&chunk.to_uppercase(), &[1]), None, "uppercase hex");
        let mut not_hex = chunk.clone();
        not_hex.replace_range(0..1, "g");
        assert_eq!(chains_hex(&not_hex, &[1]), None, "non-hex character");
        assert_eq!(chains_hex(&chunk, &[65]), None, "count 65");
        assert_eq!(chains_hex("", &[]), None, "0 chains");
        let many: String = std::iter::repeat_n(chunk.as_str(), 65).collect();
        assert_eq!(chains_hex(&many, &[1; 65]), None, "65 chains");
    }

    #[test]
    fn chains_hex_signed_refuses_negative_and_oversized_counts() {
        let chunk = hex_of("kcore-signed", 512);
        assert_eq!(chains_hex_signed(&chunk, [-1]), None, "negative count");
        assert_eq!(chains_hex_signed(&chunk, [65]), None, "count 65");
        let many: String = std::iter::repeat_n(chunk.as_str(), 65).collect();
        assert_eq!(chains_hex_signed(&many, [1; 65]), None, "65 chains");
        assert_eq!(chains_hex_signed(&chunk, [3]), Some(sdk_chains(&chunk, &[3])));
    }

    #[test]
    fn wots_address_matches_the_sdk_loop() {
        for case in 0..20 {
            let key = hex_of(&format!("kcore-key-{case}"), 8192);
            assert_eq!(wots_address(&key), Some(wots_address_sdk(&key)), "case {case}");
        }
    }

    #[test]
    fn wots_address_refuses_inputs_the_sdk_loop_must_handle() {
        let key = hex_of("kcore-key-refuse", 8192);
        assert_eq!(wots_address(&key[..2047]), None, "2047 characters");
        assert_eq!(wots_address(&key.to_uppercase()), None, "uppercase hex");
    }
}
