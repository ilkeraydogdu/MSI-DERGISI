use sha2::{Digest, Sha256};

/// Constant-time byte slice comparison to prevent timing side-channel attacks.
/// SECURITY FIX (H12): Always compares max(len_a, len_b) bytes to avoid leaking
/// hash length via early return on mismatched lengths.
/// Used for HMAC signature verification, session token validation, and secret comparison.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = std::cmp::max(a.len(), b.len());
    let mut diff: u8 = if a.len() != b.len() { 1 } else { 0 };
    for i in 0..max_len {
        let va = if i < a.len() { a[i] } else { 0 };
        let vb = if i < b.len() { b[i] } else { 0 };
        diff |= va ^ vb;
    }
    diff == 0
}

/// Computes the lowercase hexadecimal SHA-256 digest of a string slice.
/// Used to hash bearer tokens (password reset, email verification, unsubscribe)
/// before persisting or looking them up in SQLite.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_identical_slices_return_true() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different_lengths_return_false() {
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn constant_time_eq_same_length_different_content_returns_false() {
        assert!(!constant_time_eq(b"abcde", b"abcdf"));
    }

    #[test]
    fn constant_time_eq_empty_slices_return_true() {
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn sha256_hex_produces_deterministic_64_char_digest() {
        let digest = sha256_hex("msi-token-2026");
        assert_eq!(digest.len(), 64);
        assert_eq!(digest, sha256_hex("msi-token-2026"));
        assert_ne!(digest, sha256_hex("msi-token-2027"));
    }
}
