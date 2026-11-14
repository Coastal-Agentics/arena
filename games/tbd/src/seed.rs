//! Turn a caller-supplied seed into the `u64` the wind stream uses.
//!
//! A decimal digit string is the engine's seed spelling: `"42"` is 42, including
//! values that do not fit in a JS number. Anything else is FNV-1a over the
//! trimmed UTF-8 bytes, the same offset and prime as `engine::generic::StateHasher`
//! and `engine::replay::setup_hash`.

/// Decimal `u64`, or a stable FNV-1a of the text.
pub fn seed_from_text(text: &str) -> u64 {
    let t = text.trim();
    if !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(n) = t.parse::<u64>() {
            return n;
        }
    }
    fnv1a64(t.as_bytes())
}

/// A JS number used as a seed. Integers through 2^53 only; bigger integers
/// must be a bigint or a decimal string.
pub fn seed_from_f64(n: f64) -> Result<u64, String> {
    const MAX: f64 = 9007199254740991.0;
    if n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= MAX {
        Ok(n as u64)
    } else {
        Err("seed number must be an integer from 0 through 2^53".into())
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}
