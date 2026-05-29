//! PostgreSQL authentication module.
//!
//! Implements MD5 password authentication as used by the PostgreSQL
//! wire protocol v3. Supports both standard PG authentication and
//! Hologres-compatible AccessKey-based authentication.
//!
//! # MD5 Authentication Algorithm
//!
//! The PostgreSQL MD5 authentication works as follows:
//! 1. Server sends a random 4-byte salt to the client
//! 2. Client computes: `md5(md5(password + username) + salt)`
//! 3. Client prepends "md5" to the hash and sends it to the server
//! 4. Server recomputes the expected hash and compares

use md5::{Digest, Md5};
use rand::RngCore;

/// Compute the MD5 password hash for PostgreSQL authentication.
///
/// Algorithm: `"md5" + md5(md5(password + username) + salt)`
///
/// This matches the client-side computation, producing a result like:
/// `md5<hex_hash>` (e.g., `md5a1b2c3d4e5f6...`).
///
/// # Arguments
/// * `username` - The username (AccessKey ID for Hologres)
/// * `password` - The password (AccessKey Secret for Hologres)
/// * `salt` - A 4-byte random salt from the server
///
/// # Returns
/// A string in the format `md5<32_hex_chars>` representing the computed hash.
pub fn compute_md5_password(username: &str, password: &str, salt: &[u8; 4]) -> String {
    // First round: md5(password + username)
    let mut inner = Md5::new();
    inner.update(password.as_bytes());
    inner.update(username.as_bytes());
    let inner_hash = hex::encode(inner.finalize());

    // Second round: md5(inner_hash + salt)
    let mut outer = Md5::new();
    outer.update(inner_hash.as_bytes());
    outer.update(salt);
    let outer_hash = hex::encode(outer.finalize());

    format!("md5{}", outer_hash)
}

/// Verify an MD5 password response from a client.
///
/// This compares the client's response (in format `md5<hex>`) against
/// the expected hash computed from the stored password.
///
/// # Arguments
/// * `username` - The username the client is authenticating as
/// * `expected_hash` - The expected password hash in the format `md5<32_hex_chars>`
///   (this is what would be stored in `pg_authid.rolpassword` or our equivalent)
/// * `client_response` - The client's password message (in format `md5<32_hex_chars>`)
/// * `salt` - The 4-byte salt that was sent to the client
///
/// # Returns
/// `true` if the password is valid, `false` otherwise.
pub fn verify_md5_password(
    _username: &str,
    expected_hash: &str,
    client_response: &str,
    salt: &[u8; 4],
) -> bool {
    // Extract the hex portion of the expected hash (strip "md5" prefix)
    let expected_inner = expected_hash.strip_prefix("md5").unwrap_or(expected_hash);

    // Compute what the server expects: md5(stored_hash + salt)
    let mut verifier = Md5::new();
    verifier.update(expected_inner.as_bytes());
    verifier.update(salt);
    let expected_outer = hex::encode(verifier.finalize());

    // The expected full client response is "md5" + expected_outer
    let expected_response = format!("md5{}", expected_outer);
    expected_response == client_response
}

/// Generate a random 4-byte salt for MD5 authentication.
///
/// Uses a cryptographically secure random number generator.
pub fn generate_salt() -> [u8; 4] {
    let mut salt = [0u8; 4];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Configuration for password-based authentication.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether to accept any password (for development/testing).
    pub accept_any_password: bool,
    /// The expected username (AccessKey ID for Hologres).
    pub username: String,
    /// The expected password (AccessKey Secret for Hologres).
    pub password: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            accept_any_password: false,
            username: String::new(),
            password: String::new(),
        }
    }
}

/// Validate a client's password response against the expected credentials.
///
/// # Arguments
/// * `config` - The authentication configuration
/// * `username` - The username provided by the client
/// * `password_response` - The password response from the client (in "md5" or "cleartext" format)
/// * `salt` - The 4-byte salt that was sent to the client (for MD5 auth)
///
/// # Returns
/// `true` if the password is valid, `false` otherwise.
pub fn validate_password(
    config: &AuthConfig,
    username: &str,
    password_response: &str,
    salt: &[u8; 4],
) -> bool {
    if config.accept_any_password {
        return true;
    }

    if username != config.username {
        return false;
    }

    if password_response.starts_with("md5") {
        // MD5 response
        let expected_hash = compute_md5_password(username, &config.password, salt);
        expected_hash == password_response
    } else {
        // Cleartext password
        password_response == config.password
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_md5_password() {
        // Test with known values
        let username = "testuser";
        let password = "testpass";
        let salt = [0x12, 0x34, 0x56, 0x78];

        let result = compute_md5_password(username, password, &salt);

        // Verify format: starts with "md5"
        assert!(result.starts_with("md5"));
        // Verify length: "md5" + 32 hex chars = 35
        assert_eq!(result.len(), 35);

        // Verify it's deterministic
        let result2 = compute_md5_password(username, password, &salt);
        assert_eq!(result, result2);
    }

    #[test]
    fn test_compute_md5_with_different_salts() {
        let username = "user";
        let password = "pass";

        let salt1 = [0x00, 0x00, 0x00, 0x00];
        let salt2 = [0xff, 0xff, 0xff, 0xff];

        let result1 = compute_md5_password(username, password, &salt1);
        let result2 = compute_md5_password(username, password, &salt2);

        // Different salts should produce different results
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_verify_md5_password_valid() {
        let username = "testuser";
        let password = "testpass";
        let salt = [0x12, 0x34, 0x56, 0x78];

        // Compute what the server would store (same as client computes)
        let stored_hash = compute_md5_password(username, password, &salt);
        // The "expected hash" stored in the server is md5(password+username)
        let expected_inner = {
            let mut inner = Md5::new();
            inner.update(password.as_bytes());
            inner.update(username.as_bytes());
            format!("md5{}", hex::encode(inner.finalize()))
        };

        // The client would send: md5(md5(password+username) + salt) = stored_hash
        assert!(verify_md5_password(
            username,
            &expected_inner,
            &stored_hash,
            &salt
        ));
    }

    #[test]
    fn test_verify_md5_password_invalid() {
        let username = "testuser";
        let salt = [0x12, 0x34, 0x56, 0x78];

        let wrong_expected = format!("md5{}", "a".repeat(32));
        let wrong_client = format!("md5{}", "b".repeat(32));

        assert!(!verify_md5_password(
            username,
            &wrong_expected,
            &wrong_client,
            &salt
        ));
    }

    #[test]
    fn test_generate_salt() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();

        assert_eq!(salt1.len(), 4);
        assert_eq!(salt2.len(), 4);
        // Very unlikely to be the same
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_validate_password_accept_any() {
        let config = AuthConfig {
            accept_any_password: true,
            username: "anyuser".to_string(),
            password: "".to_string(),
        };
        let salt = [0x00, 0x00, 0x00, 0x00];

        assert!(validate_password(&config, "anyuser", "wrongpass", &salt));
    }

    #[test]
    fn test_validate_password_wrong_username() {
        let config = AuthConfig {
            accept_any_password: false,
            username: "correctuser".to_string(),
            password: "correctpass".to_string(),
        };
        let salt = [0x00, 0x00, 0x00, 0x00];

        assert!(!validate_password(
            &config,
            "wronguser",
            "correctpass",
            &salt
        ));
    }

    #[test]
    fn test_validate_password_cleartext() {
        let config = AuthConfig {
            accept_any_password: false,
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        let salt = [0x00, 0x00, 0x00, 0x00];

        assert!(validate_password(&config, "user", "pass", &salt));
        assert!(!validate_password(&config, "user", "wrong", &salt));
    }

    #[test]
    fn test_validate_password_md5() {
        let config = AuthConfig {
            accept_any_password: false,
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        let salt = generate_salt();

        // Compute the correct MD5 response
        let correct_response = compute_md5_password("user", "pass", &salt);
        assert!(validate_password(&config, "user", &correct_response, &salt));

        // Wrong MD5 response should fail
        assert!(!validate_password(
            &config,
            "user",
            "md5wronghash1234567890abcdef0123456",
            &salt
        ));
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.accept_any_password);
        assert!(config.username.is_empty());
        assert!(config.password.is_empty());
    }
}
