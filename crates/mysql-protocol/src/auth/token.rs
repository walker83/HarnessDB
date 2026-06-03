use async_trait::async_trait;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::{AuthError, AuthPlugin, AuthPluginType, AuthUser};

type HmacSha256 = Hmac<Sha256>;

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

pub struct TokenAuth {
    config: TokenConfig,
}

impl TokenAuth {
    pub fn new(config: TokenConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AuthPlugin for TokenAuth {
    fn plugin_type(&self) -> AuthPluginType {
        AuthPluginType::Token
    }

    async fn authenticate(
        &self,
        username: &str,
        auth_response: &[u8],
        _auth_plugin_data: &[u8],
    ) -> Result<AuthUser, AuthError> {
        if username.is_empty() {
            return Err(AuthError::Failed("Empty username".to_string()));
        }

        let token = String::from_utf8_lossy(auth_response);
        match validate_jwt_token(&token, &self.config.secret) {
            Ok(claims) => {
                if claims.username != username {
                    return Err(AuthError::Failed("Token username mismatch".to_string()));
                }
                Ok(AuthUser {
                    username: claims.username,
                    hostname: None,
                    roles: claims.roles,
                    auth_plugin: AuthPluginType::Token,
                })
            }
            Err(e) => Err(AuthError::Failed(format!("Invalid token: {}", e))),
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.exp < now
    }
}

pub fn generate_jwt_token(claims: &JwtClaims, secret: &str) -> Result<String, AuthError> {
    let header = base64_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let payload = base64_encode(
        serde_json::to_vec(claims)
            .map_err(|e| AuthError::Failed(e.to_string()))?
            .as_slice(),
    );

    let signature = {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| AuthError::Failed(e.to_string()))?;
        mac.update(format!("{}.{}", header, payload).as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    Ok(format!("{}.{}.{}", header, payload, signature))
}

pub fn validate_jwt_token(token: &str, secret: &str) -> Result<JwtClaims, AuthError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::Failed("Invalid token format".to_string()));
    }

    let signature = {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| AuthError::Failed(e.to_string()))?;
        mac.update(format!("{}.{}", parts[0], parts[1]).as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    if signature != parts[2] {
        return Err(AuthError::Failed("Invalid signature".to_string()));
    }

    let payload_bytes = base64_decode(parts[1])?;
    let claims: JwtClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| AuthError::Failed(format!("Invalid claims: {}", e)))?;

    if claims.is_expired() {
        return Err(AuthError::Failed("Token expired".to_string()));
    }

    Ok(claims)
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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

fn base64_decode(input: &str) -> Result<Vec<u8>, AuthError> {
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
            _ => return Err(AuthError::Failed("Invalid base64 character".to_string())),
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
            "harnessdb".to_string(),
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
            "harnessdb".to_string(),
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
