use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    pub secret: String,
    pub expiry_secs: u64,
    pub issuer: String,
}

impl TokenConfig {
    pub fn new(secret: String, expiry_secs: u64, issuer: String) -> Self {
        Self {
            secret,
            expiry_secs,
            issuer,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub username: String,
    pub roles: Vec<String>,
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
}

impl JwtClaims {
    pub fn new(username: String, roles: Vec<String>, expiry_secs: u64, issuer: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            username,
            roles,
            exp: now + expiry_secs,
            iat: now,
            iss: issuer,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.exp < now
    }
}

pub fn generate_jwt_token(claims: &JwtClaims, secret: &str) -> Result<String, String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let header = base64_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let payload = base64_encode(
        serde_json::to_vec(claims).map_err(|e| format!("JSON error: {}", e))?.as_slice(),
    );

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(format!("{}.{}", header, payload).as_bytes());
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());

    Ok(format!("{}.{}.{}", header, payload, signature))
}

pub fn validate_jwt_token(token: &str, secret: &str) -> Result<JwtClaims, String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format".to_string());
    }

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(format!("{}.{}", parts[0], parts[1]).as_bytes());
    let result = mac.finalize();
    let expected_sig = hex::encode(result.into_bytes());

    if expected_sig != parts[2] {
        return Err("Invalid signature".to_string());
    }

    let payload_bytes = base64_decode(parts[1])?;
    let claims: JwtClaims =
        serde_json::from_slice(&payload_bytes).map_err(|e| format!("Invalid claims: {}", e))?;

    if claims.is_expired() {
        return Err("Token expired".to_string());
    }

    Ok(claims)
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for c in input.chars() {
        let val = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return Err(format!("Invalid base64 character: {}", c)),
        };

        buffer = (buffer << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_claims() {
        let claims = JwtClaims::new(
            "user1".to_string(),
            vec!["role1".to_string()],
            3600,
            "rorisdb".to_string(),
        );
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_generate_and_validate_token() {
        let secret = "test_secret_key";
        let claims = JwtClaims::new(
            "testuser".to_string(),
            vec!["admin".to_string()],
            3600,
            "rorisdb".to_string(),
        );

        let token = generate_jwt_token(&claims, secret).unwrap();
        let validated = validate_jwt_token(&token, secret).unwrap();
        assert_eq!(validated.username, "testuser");
        assert_eq!(validated.roles, vec!["admin"]);
    }

    #[test]
    fn test_invalid_token() {
        let result = validate_jwt_token("invalid.token.here", "secret");
        assert!(result.is_err());
    }
}