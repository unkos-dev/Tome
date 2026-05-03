use base64ct::{Base64UrlUnpadded, Encoding};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

fn sha256_hex(input: &[u8]) -> String {
    Sha256::digest(input)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Generate a cryptographically random device token (32 bytes, base64url).
/// Returns (`plaintext_token`, `sha256_hex_hash`).
pub fn generate_device_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let plaintext = Base64UrlUnpadded::encode_string(&bytes);
    let hash = sha256_hex(plaintext.as_bytes());
    (plaintext, hash)
}

/// Verify a plaintext token against a stored SHA-256 hex hash.
/// Uses constant-time comparison to prevent timing attacks.
pub fn verify_device_token(plaintext: &str, hash: &str) -> bool {
    let computed = sha256_hex(plaintext.as_bytes());
    computed.as_bytes().ct_eq(hash.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_device_token_format() {
        let (plaintext, hash) = generate_device_token();
        // 32 bytes base64url unpadded = 43 chars
        assert_eq!(plaintext.len(), 43);
        // SHA-256 hex = 64 lowercase hex chars
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_correct_token() {
        let (plaintext, hash) = generate_device_token();
        assert!(verify_device_token(&plaintext, &hash));
    }

    #[test]
    fn verify_wrong_token() {
        let (_plaintext, hash) = generate_device_token();
        assert!(!verify_device_token("wrong-token", &hash));
    }

    #[test]
    fn verify_malformed_hash() {
        assert!(!verify_device_token("any-token", "not-a-valid-hash"));
    }
}
