//! PKCE (Proof Key for Code Exchange) implementation for OAuth 2.0.
//!
//! Implements RFC 7636 for secure authorization code exchange using the S256
//! (SHA-256) challenge method.
//!
//! # Example
//!
//! ```rust
//! use codex_core::provider_auth::pkce;
//!
//! let verifier = pkce::generate_code_verifier();
//! let challenge = pkce::generate_code_challenge(&verifier);
//! let state = pkce::generate_state();
//!
//! // Use challenge in authorization URL, verifier in token exchange
//! ```

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// Generates a cryptographically random code verifier for PKCE.
///
/// The verifier is a 32-byte random value encoded as base64url without padding,
/// resulting in a 43-character string that complies with RFC 7636.
///
/// # Returns
///
/// A base64url-encoded string suitable for use as a PKCE code verifier.
pub fn generate_code_verifier() -> String {
    let random_bytes: [u8; 32] = rand::random();
    URL_SAFE_NO_PAD.encode(random_bytes)
}

/// Generates a code challenge from a verifier using the S256 method.
///
/// Computes `BASE64URL(SHA256(verifier))` as specified in RFC 7636.
///
/// # Arguments
///
/// * `verifier` - The code verifier string
///
/// # Returns
///
/// A base64url-encoded SHA-256 hash of the verifier.
pub fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generates a random state parameter for CSRF protection.
///
/// The state parameter prevents cross-site request forgery by ensuring the
/// OAuth callback corresponds to a request initiated by this application.
///
/// # Returns
///
/// A base64url-encoded 16-byte random string.
pub fn generate_state() -> String {
    let random_bytes: [u8; 16] = rand::random();
    URL_SAFE_NO_PAD.encode(random_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let verifier = generate_code_verifier();
        // 32 bytes * 4/3 base64 = 43 characters (without padding)
        assert_eq!(verifier.len(), 43);
    }

    #[test]
    fn test_code_verifier_uniqueness() {
        let v1 = generate_code_verifier();
        let v2 = generate_code_verifier();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_code_verifier_valid_base64url() {
        let verifier = generate_code_verifier();
        // Should only contain base64url characters (A-Z, a-z, 0-9, -, _)
        assert!(
            verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn test_code_challenge_length() {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        // SHA-256 = 32 bytes * 4/3 base64 = 43 characters (without padding)
        assert_eq!(challenge.len(), 43);
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let verifier = "test-verifier-string";
        let c1 = generate_code_challenge(verifier);
        let c2 = generate_code_challenge(verifier);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_code_challenge_known_value() {
        // Test against known S256 PKCE challenge
        // verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        // Expected challenge based on RFC 7636 example
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = generate_code_challenge(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn test_state_length() {
        let state = generate_state();
        // 16 bytes * 4/3 base64 = 22 characters (without padding)
        assert_eq!(state.len(), 22);
    }

    #[test]
    fn test_state_uniqueness() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_state_valid_base64url() {
        let state = generate_state();
        assert!(
            state
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }
}
