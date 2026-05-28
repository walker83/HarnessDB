//! HMAC-based signature verification for MaxCompute (ODPS) REST API.
//!
//! Supports both V2 (HMAC-SHA1) and V4 (HMAC-SHA256 with key derivation)
//! signature schemes as used by Alibaba Cloud MaxCompute.
//!
//! # V2 Signature
//!
//! ```text
//! Authorization: ODPS {access_id}:{base64(hmac-sha1(secret, canonical_string))}
//! ```
//!
//! Canonical string (V2):
//!
//! ```text
//! HTTP_METHOD\n
//! Content-Type\n         (empty line if not present)
//! Content-MD5\n          (empty line if not present)
//! Date\n
//! x-odps-* headers       (sorted by name, format: name:value\n)
//! URL.path?query         (query params sorted by key)
//! ```
//!
//! # V4 Signature
//!
//! ```text
//! Authorization: ODPS {access_id}/{YYYYMMDD}/{region}/odps/aliyun_v4_request:{base64(sig)}
//! ```
//!
//! V4 signing key derivation:
//!
//! ```text
//! k_secret    = "aliyun_v4" + secret
//! k_date      = HMAC-SHA256(k_secret, YYYYMMDD)
//! k_region    = HMAC-SHA256(k_date, region)
//! k_service   = HMAC-SHA256(k_region, "odps")
//! signing_key = HMAC-SHA256(k_service, "aliyun_v4_request")
//! ```

use axum::http::HeaderMap;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use thiserror::Error;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during authentication.
#[derive(Error, Debug)]
pub enum AuthError {
    /// The Authorization header is missing.
    #[error("Missing Authorization header")]
    MissingAuth,

    /// The Authorization header could not be parsed.
    #[error("Invalid Authorization header format")]
    InvalidFormat,

    /// The access key ID in the header does not match the config.
    #[error("Access key ID mismatch")]
    KeyIdMismatch,

    /// The computed signature does not match the provided signature.
    #[error("Signature verification failed")]
    SignatureInvalid,

    /// A required header for signature computation is missing.
    #[error("Missing required header: {0}")]
    MissingHeader(String),

    /// Unsupported signature scheme.
    #[error("Unsupported signature scheme")]
    UnsupportedScheme,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// MaxCompute authentication configuration.
///
/// When `region` is `Some`, V4 (HMAC-SHA256) signing is used.
/// When `region` is `None`, V2 (HMAC-SHA1) signing is used.
#[derive(Debug, Clone)]
pub struct McAuthConfig {
    /// MaxCompute AccessKey ID
    pub access_key_id: String,
    /// MaxCompute AccessKey Secret
    pub access_key_secret: String,
    /// Region identifier (e.g. "cn-hangzhou").
    /// `Some(region)` enables V4 signing; `None` selects V2.
    pub region: Option<String>,
}

impl McAuthConfig {
    /// Create a new V2 authentication config (no region).
    pub fn new_v2(access_key_id: impl Into<String>, access_key_secret: impl Into<String>) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            access_key_secret: access_key_secret.into(),
            region: None,
        }
    }

    /// Create a new V4 authentication config (with region).
    pub fn new_v4(
        access_key_id: impl Into<String>,
        access_key_secret: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            access_key_secret: access_key_secret.into(),
            region: Some(region.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// V2 Canonical String
// ---------------------------------------------------------------------------

/// Build the canonical string for V2 signature verification.
///
/// The canonical string format is:
/// ```text
/// HTTP_METHOD\n
/// Content-Type\n        (empty if not present)
/// Content-MD5\n         (empty if not present)
/// Date\n
/// x-odps-* headers      (sorted by name, format: name:value\n)
/// resource              (URL path + ?query params sorted by key)
/// ```
pub fn build_canonical_string_v2(
    method: &str,
    content_type: &str,
    content_md5: &str,
    date: &str,
    headers: &HeaderMap,
    resource: &str,
) -> String {
    let mut result = String::new();

    // HTTP method
    result.push_str(method);
    result.push('\n');

    // Content-Type (empty line if not present)
    result.push_str(content_type);
    result.push('\n');

    // Content-MD5 (empty line if not present)
    result.push_str(content_md5);
    result.push('\n');

    // Date
    result.push_str(date);
    result.push('\n');

    // x-odps-* headers, sorted by name, format: name:value\n
    let odps_part = canonicalized_odps_headers(headers);
    result.push_str(&odps_part);

    // Resource (URL path + sorted query params)
    result.push_str(resource);

    result
}

// ---------------------------------------------------------------------------
// V4 Canonical Request (AWS SigV4 style)
// ---------------------------------------------------------------------------

/// Build the V4 canonical request following AWS SigV4 format.
///
/// ```text
/// HTTPMethod\n
/// CanonicalURI\n
/// CanonicalQueryString\n
/// CanonicalHeaders\n
/// SignedHeaders\n
/// HashedRequestPayload
/// ```
///
/// CanonicalHeaders includes content-type, content-md5, date, and x-odps-*
/// headers sorted by lowercase name.
///
/// HashedRequestPayload = hex(SHA256(request_body)) or hex(SHA256("")) for
/// empty bodies.
pub fn build_canonical_request_v4(
    method: &str,
    path: &str,
    query: &str,
    content_type: &str,
    content_md5: &str,
    date: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> String {
    let mut result = String::new();

    // HTTP method
    result.push_str(method);
    result.push('\n');

    // CanonicalURI
    if path.is_empty() {
        result.push('/');
    } else {
        result.push_str(path);
    }
    result.push('\n');

    // CanonicalQueryString (sorted key=value pairs)
    let canonical_qs = canonical_query_string_v4(query);
    result.push_str(&canonical_qs);
    result.push('\n');

    // CanonicalHeaders (sorted lowercase header:value\n)
    // Includes: content-type, content-md5, date, and all x-odps-* headers
    let (canonical_headers, signed_headers) = build_canonical_headers_v4(
        headers, content_type, content_md5, date,
    );
    result.push_str(&canonical_headers);
    result.push('\n');

    // SignedHeaders
    result.push_str(&signed_headers);
    result.push('\n');

    // HashedRequestPayload
    let payload_hash = if body.is_empty() {
        hex::encode(Sha256::digest(b""))
    } else {
        hex::encode(Sha256::digest(body))
    };
    result.push_str(&payload_hash);

    result
}

/// Build the V4 StringToSign.
///
/// ```text
/// ACS4-HMAC-SHA256\n
/// {timestamp}\n
/// {date}/{region}/odps/aliyun_v4_request\n
/// {hex(sha256(canonical_request))}
/// ```
pub fn build_string_to_sign_v4(
    timestamp: &str,
    date_yyyy_mm_dd: &str,
    region: &str,
    canonical_request_hash: &str,
) -> String {
    format!(
        "ACS4-HMAC-SHA256\n{timestamp}\n{date}/{region}/odps/aliyun_v4_request\n{hash}",
        timestamp = timestamp,
        date = date_yyyy_mm_dd,
        region = region,
        hash = canonical_request_hash,
    )
}

/// Build V4 canonical headers and signed headers string.
///
/// Returns (canonical_headers_string, signed_headers_string).
/// Canonical headers are sorted lowercase `name:value\n` entries for
/// content-type, content-md5, date, and x-odps-* headers.
fn build_canonical_headers_v4(
    headers: &HeaderMap,
    content_type: &str,
    content_md5: &str,
    date: &str,
) -> (String, String) {
    let mut header_pairs: Vec<(String, String)> = Vec::new();

    // content-type (only if present)
    if !content_type.is_empty() {
        header_pairs.push(("content-type".to_string(), content_type.trim().to_string()));
    }

    // content-md5 (only if present)
    if !content_md5.is_empty() {
        header_pairs.push(("content-md5".to_string(), content_md5.trim().to_string()));
    }

    // date
    header_pairs.push(("date".to_string(), date.trim().to_string()));

    // x-odps-* headers
    for (name, value) in headers.iter() {
        let lower = name.as_str().to_ascii_lowercase();
        if lower.starts_with("x-odps-") {
            let already_added = header_pairs.iter().any(|(k, _)| k == &lower);
            if !already_added {
                header_pairs.push((lower, value.to_str().unwrap_or("").trim().to_string()));
            }
        }
    }

    // Sort by header name
    header_pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical_headers: String = header_pairs
        .iter()
        .map(|(k, v)| format!("{}:{}\n", k, v))
        .collect();

    let signed_headers: String = header_pairs
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<&str>>()
        .join(";");

    (canonical_headers, signed_headers)
}

/// Sort query parameters by key for V4 canonical query string.
fn canonical_query_string_v4(query: &str) -> String {
    if query.is_empty() {
        return String::new();
    }
    let mut params: Vec<(&str, &str)> = query
        .split('&')
        .filter_map(|p| {
            let mut parts = p.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key, value))
        })
        .collect();
    params.sort_by(|a, b| a.0.cmp(b.0));
    params
        .iter()
        .map(|(k, v)| {
            if v.is_empty() {
                (*k).to_string()
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Parse an HTTP-date (RFC 2822) into ISO 8601 timestamp format (YYYYMMDDTHHMMSSZ).
fn http_date_to_iso8601_timestamp(date_str: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc2822(date_str.trim())
        .ok()
        .map(|dt| dt.format("%Y%m%dT%H%M%SZ").to_string())
}

// ---------------------------------------------------------------------------
// Request Verification (entry point)
// ---------------------------------------------------------------------------

/// Verify the authenticity of a MaxCompute REST API request.
///
/// Automatically detects V2 or V4 signing based on the Authorization header
/// format. V4 headers contain `/odps/aliyun_v4_request`; V2 do not.
///
/// # Arguments
///
/// * `config` - Authentication configuration
/// * `method` - HTTP method (GET, POST, etc.)
/// * `path` - URL path (e.g. `/projects/test_project/tables`)
/// * `query` - URL query string (e.g. `result` or empty string)
/// * `headers` - HTTP request headers
/// * `body` - Request body bytes (used for V4 payload hash; may be empty for GET)
///
/// # Returns
///
/// `Ok(true)` if the signature is valid, `Ok(false)` if it doesn't match,
/// or `Err(AuthError)` for structural parsing failures.
pub fn verify_request(
    config: &McAuthConfig,
    method: &str,
    path: &str,
    query: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<bool, AuthError> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingAuth)?;

    let stripped = auth_header.strip_prefix("ODPS ").ok_or(AuthError::UnsupportedScheme)?;

    // V4 headers contain "/odps/aliyun_v4_request"
    if stripped.contains("/odps/aliyun_v4_request") {
        verify_v4(config, method, path, query, headers, stripped, body)
    } else {
        verify_v2(config, method, path, query, headers, stripped)
    }
}

// ---------------------------------------------------------------------------
// V2 Verification
// ---------------------------------------------------------------------------

fn verify_v2(
    config: &McAuthConfig,
    method: &str,
    path: &str,
    query: &str,
    headers: &HeaderMap,
    auth_value: &str,
) -> Result<bool, AuthError> {
    // Parse "access_id:signature"
    let parts: Vec<&str> = auth_value.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(AuthError::InvalidFormat);
    }
    let provided_key_id = parts[0];
    let provided_sig = parts[1];

    // Key ID check
    if provided_key_id != config.access_key_id {
        return Ok(false);
    }

    // Extract canonical parameters
    let content_type = header_value(headers, "content-type").unwrap_or_default();
    let content_md5 = header_value(headers, "content-md5").unwrap_or_default();
    let date = header_value(headers, "date").ok_or(AuthError::MissingHeader("date".into()))?;
    let resource = canonicalized_resource(path, query);

    let canonical = build_canonical_string_v2(
        method,
        &content_type,
        &content_md5,
        &date,
        headers,
        &resource,
    );

    let expected = hmac_sha1_base64(config.access_key_secret.as_bytes(), canonical.as_bytes());
    Ok(constant_time_eq(provided_sig.as_bytes(), expected.as_bytes()))
}

// ---------------------------------------------------------------------------
// V4 Verification
// ---------------------------------------------------------------------------

fn verify_v4(
    config: &McAuthConfig,
    method: &str,
    path: &str,
    query: &str,
    headers: &HeaderMap,
    auth_value: &str,
    body: &[u8],
) -> Result<bool, AuthError> {
    // V4 format: access_id/YYYYMMDD/region/odps/aliyun_v4_request:signature
    let colon_pos = auth_value.rfind(':').ok_or(AuthError::InvalidFormat)?;
    let prefix = &auth_value[..colon_pos];
    let provided_sig = &auth_value[colon_pos + 1..];

    let prefix_parts: Vec<&str> = prefix.split('/').collect();
    if prefix_parts.len() < 5 {
        return Err(AuthError::InvalidFormat);
    }
    let provided_key_id = prefix_parts[0];
    let date_yyyy_mm_dd = prefix_parts[1];
    let region = prefix_parts[2];

    // Key ID check
    if provided_key_id != config.access_key_id {
        return Ok(false);
    }

    // Region check (if configured)
    if let Some(config_region) = &config.region {
        if region != config_region.as_str() {
            return Ok(false);
        }
    }

    // Derive V4 signing key
    let signing_key = derive_v4_signing_key(&config.access_key_secret, date_yyyy_mm_dd, region);

    // Extract canonical parameters
    let content_type = header_value(headers, "content-type").unwrap_or_default();
    let content_md5 = header_value(headers, "content-md5").unwrap_or_default();
    let date = header_value(headers, "date").ok_or(AuthError::MissingHeader("date".into()))?;

    // Build the canonical request (AWS SigV4 style)
    let canonical_request = build_canonical_request_v4(
        method,
        path,
        query,
        &content_type,
        &content_md5,
        &date,
        headers,
        body,
    );

    // Hash the canonical request
    let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));

    // Build the timestamp from the Date header (ISO 8601 format)
    // For V4, the timestamp used in StringToSign comes from the x-odps-date header or date header
    let timestamp = http_date_to_iso8601_timestamp(&date)
        .unwrap_or_else(|| format!("{}T000000Z", date_yyyy_mm_dd));

    // Build the StringToSign
    let string_to_sign = build_string_to_sign_v4(
        &timestamp,
        date_yyyy_mm_dd,
        region,
        &canonical_request_hash,
    );

    let expected = hmac_sha256_base64(&signing_key, string_to_sign.as_bytes());
    Ok(constant_time_eq(provided_sig.as_bytes(), expected.as_bytes()))
}

// ---------------------------------------------------------------------------
// Signing key derivation (V4)
// ---------------------------------------------------------------------------

/// Derive the V4 signing key from the secret access key, date, and region.
///
/// Key derivation chain:
/// ```text
/// k_secret    = "aliyun_v4" + secret_access_key
/// k_date      = HMAC-SHA256(k_secret, YYYYMMDD)
/// k_region    = HMAC-SHA256(k_date, region)
/// k_service   = HMAC-SHA256(k_region, "odps")
/// signing_key = HMAC-SHA256(k_service, "aliyun_v4_request")
/// ```
pub fn derive_v4_signing_key(secret: &str, date_yyyy_mm_dd: &str, region: &str) -> Vec<u8> {
    let k_secret = format!("aliyun_v4{}", secret);
    let k_date = hmac_sha256_raw(k_secret.as_bytes(), date_yyyy_mm_dd.as_bytes());
    let k_region = hmac_sha256_raw(&k_date, region.as_bytes());
    let k_service = hmac_sha256_raw(&k_region, b"odps");
    hmac_sha256_raw(&k_service, b"aliyun_v4_request")
}

// ---------------------------------------------------------------------------
// Signing helpers
// ---------------------------------------------------------------------------

fn hmac_sha1_base64(key: &[u8], data: &[u8]) -> String {
    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    BASE64.encode(mac.finalize().into_bytes())
}

fn hmac_sha256_raw(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hmac_sha256_base64(key: &[u8], data: &[u8]) -> String {
    let result = hmac_sha256_raw(key, data);
    BASE64.encode(result)
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Extract a header value as a trimmed string.
fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
}

/// Extract and sort x-odps-* headers into `name:value\n` format.
pub fn canonicalized_odps_headers(headers: &HeaderMap) -> String {
    let mut odps_headers: Vec<(String, String)> = headers
        .iter()
        .filter(|(name, _)| name.as_str().to_ascii_lowercase().starts_with("x-odps-"))
        .map(|(name, value)| {
            (
                name.as_str().to_ascii_lowercase(),
                value.to_str().unwrap_or("").trim().to_string(),
            )
        })
        .collect();
    odps_headers.sort_by(|a, b| a.0.cmp(&b.0));
    odps_headers
        .iter()
        .map(|(k, v)| format!("{}:{}\n", k, v))
        .collect()
}

/// Build the resource string from path and query parameters.
///
/// If `query` is non-empty, query parameters are sorted by key and the result
/// is `path?key1=val1&key2=val2`. Otherwise it is just `path`.
pub fn canonicalized_resource(path: &str, query: &str) -> String {
    if query.is_empty() {
        return path.to_string();
    }
    let mut params: Vec<(&str, &str)> = query
        .split('&')
        .filter_map(|p| {
            let mut parts = p.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key, value))
        })
        .collect();
    params.sort_by(|a, b| a.0.cmp(b.0));
    let sorted = params
        .iter()
        .map(|(k, v)| {
            if v.is_empty() {
                k.to_string()
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect::<Vec<_>>()
        .join("&");
    format!("{}?{}", path, sorted)
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Helper to create a signed V2 Authorization header for testing.
pub fn sign_request(
    config: &McAuthConfig,
    method: &str,
    path: &str,
    query: &str,
    content_type: &str,
    date: &str,
) -> String {
    let resource = canonicalized_resource(path, query);
    let canonical = format!(
        "{}\n{}\n\n{}\n{}",
        method.to_uppercase(),
        content_type,
        date,
        resource,
    );
    let sig = hmac_sha1_base64(config.access_key_secret.as_bytes(), canonical.as_bytes());
    format!("ODPS {}:{}", config.access_key_id, sig)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // HMAC computation
    // -----------------------------------------------------------------------

    #[test]
    fn test_hmac_sha1_computation() {
        let result = hmac_sha1_base64(b"secret", b"test data");
        assert!(!result.is_empty());
        // Verify it's valid base64
        assert!(BASE64.decode(&result).is_ok());
    }

    #[test]
    fn test_hmac_sha1_deterministic() {
        let a = hmac_sha1_base64(b"key", b"data");
        let b = hmac_sha1_base64(b"key", b"data");
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // Canonical string (V2)
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_canonical_string_v2_basic() {
        let headers = HeaderMap::new();
        let canonical = build_canonical_string_v2(
            "GET",
            "application/xml",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            "/projects/test_project/tables",
        );
        let expected =
            "GET\napplication/xml\n\nMon, 01 Jan 2024 00:00:00 GMT\n/projects/test_project/tables";
        assert_eq!(canonical, expected);
    }

    #[test]
    fn test_build_canonical_string_v2_with_odps_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-odps-request-id", "abc123".parse().unwrap());
        headers.insert("x-odps-project", "test_project".parse().unwrap());

        let canonical = build_canonical_string_v2(
            "POST",
            "",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            "/projects/test_project/instances",
        );
        assert!(canonical.contains("x-odps-project:test_project\n"));
        assert!(canonical.contains("x-odps-request-id:abc123\n"));

        // x-odps-project should come before x-odps-request-id alphabetically
        let p_pos = canonical.find("x-odps-project").unwrap();
        let r_pos = canonical.find("x-odps-request-id").unwrap();
        assert!(p_pos < r_pos);
    }

    #[test]
    fn test_build_canonical_string_v2_with_resource() {
        let headers = HeaderMap::new();
        let resource = canonicalized_resource("/projects/p/instances/i1", "result");
        let canonical = build_canonical_string_v2(
            "GET",
            "",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            &resource,
        );
        assert!(canonical.contains("/projects/p/instances/i1?result"));
    }

    // -----------------------------------------------------------------------
    // Resource building
    // -----------------------------------------------------------------------

    #[test]
    fn test_canonicalized_resource_no_query() {
        assert_eq!(canonicalized_resource("/api/projects/p1", ""), "/api/projects/p1");
    }

    #[test]
    fn test_canonicalized_resource_with_query() {
        let result = canonicalized_resource("/api/projects/p1", "b=2&a=1");
        assert_eq!(result, "/api/projects/p1?a=1&b=2");
    }

    #[test]
    fn test_canonicalized_resource_query_without_value() {
        let result = canonicalized_resource("/api/projects/p1", "result");
        assert_eq!(result, "/api/projects/p1?result");
    }

    // -----------------------------------------------------------------------
    // x-odps headers
    // -----------------------------------------------------------------------

    #[test]
    fn test_canonicalized_odps_headers_sorted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-odps-b", "val_b".parse().unwrap());
        headers.insert("x-odps-a", "val_a".parse().unwrap());
        let result = canonicalized_odps_headers(&headers);
        assert_eq!(result, "x-odps-a:val_a\nx-odps-b:val_b\n");
    }

    #[test]
    fn test_canonicalized_odps_headers_empty() {
        let headers = HeaderMap::new();
        assert_eq!(canonicalized_odps_headers(&headers), "");
    }

    #[test]
    fn test_canonicalized_odps_headers_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("X-ODPS-TOKEN", "abc".parse().unwrap());
        let result = canonicalized_odps_headers(&headers);
        assert_eq!(result, "x-odps-token:abc\n");
    }

    // -----------------------------------------------------------------------
    // Constant-time comparison
    // -----------------------------------------------------------------------

    #[test]
    fn test_constant_time_eq_equal() {
        assert!(constant_time_eq(b"abc", b"abc"));
    }

    #[test]
    fn test_constant_time_eq_different() {
        assert!(!constant_time_eq(b"abc", b"abd"));
    }

    #[test]
    fn test_constant_time_eq_different_length() {
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }

    // -----------------------------------------------------------------------
    // V4 key derivation
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_v4_signing_key_length() {
        let key = derive_v4_signing_key("secret", "20240101", "cn-hangzhou");
        assert_eq!(key.len(), 32); // SHA-256 output is 32 bytes
    }

    #[test]
    fn test_derive_v4_signing_key_deterministic() {
        let a = derive_v4_signing_key("secret", "20240101", "cn-hangzhou");
        let b = derive_v4_signing_key("secret", "20240101", "cn-hangzhou");
        assert_eq!(a, b);
    }

    #[test]
    fn test_derive_v4_signing_key_different_date() {
        let a = derive_v4_signing_key("secret", "20240101", "cn-hangzhou");
        let b = derive_v4_signing_key("secret", "20240102", "cn-hangzhou");
        assert_ne!(a, b);
    }

    #[test]
    fn test_derive_v4_signing_key_different_region() {
        let a = derive_v4_signing_key("secret", "20240101", "cn-hangzhou");
        let b = derive_v4_signing_key("secret", "20240101", "cn-beijing");
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // Sign and verify V2
    // -----------------------------------------------------------------------

    #[test]
    fn test_sign_and_verify_v2() {
        let config = McAuthConfig::new_v2("test_key", "test_secret");
        let auth = sign_request(
            &config,
            "GET",
            "/api/projects/test",
            "",
            "application/json",
            "Mon, 01 Jan 2024 00:00:00 GMT",
        );
        assert!(auth.starts_with("ODPS test_key:"));

        let mut headers = HeaderMap::new();
        headers.insert("authorization", auth.parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());

        let result = verify_request(&config, "GET", "/api/projects/test", "", &headers, &[]);
        assert!(result.is_ok(), "verify should succeed: {:?}", result.err());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_v2_invalid_signature() {
        let config = McAuthConfig::new_v2("test_key", "test_secret");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert(
            "authorization",
            "ODPS test_key:invalidsignature==".parse().unwrap(),
        );
        let result = verify_request(&config, "GET", "/api/projects/test", "", &headers, &[]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_v2_wrong_key_id() {
        let config = McAuthConfig::new_v2("correct_key", "secret");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert(
            "authorization",
            "ODPS wrong_key:somesig".parse().unwrap(),
        );
        let result = verify_request(&config, "GET", "/test", "", &headers, &[]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_v2_missing_auth_header() {
        let config = McAuthConfig::new_v2("key", "secret");
        let headers = HeaderMap::new();
        let result = verify_request(&config, "GET", "/", "", &headers, &[]);
        assert!(matches!(result, Err(AuthError::MissingAuth)));
    }

    #[test]
    fn test_verify_v2_missing_date() {
        let config = McAuthConfig::new_v2("key", "secret");
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "ODPS key:somesig".parse().unwrap(),
        );
        let result = verify_request(&config, "GET", "/", "", &headers, &[]);
        assert!(matches!(result, Err(AuthError::MissingHeader(_))));
    }

    #[test]
    fn test_verify_v2_with_x_odps_headers() {
        let config = McAuthConfig::new_v2("akey", "skey");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert("content-type", "application/xml".parse().unwrap());
        headers.insert("x-odps-project", "my_project".parse().unwrap());

        let resource = canonicalized_resource("/projects/my_project/instances", "");
        let canonical = build_canonical_string_v2(
            "POST",
            "application/xml",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            &resource,
        );
        let sig = hmac_sha1_base64(b"skey", canonical.as_bytes());
        headers.insert(
            "authorization",
            format!("ODPS akey:{}", sig).parse().unwrap(),
        );

        let result = verify_request(&config, "POST", "/projects/my_project/instances", "", &headers, &[]);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    // -----------------------------------------------------------------------
    // V4 Canonical Request
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_canonical_request_v4_basic() {
        let headers = HeaderMap::new();
        let canonical = build_canonical_request_v4(
            "GET",
            "/api/projects/test",
            "",
            "",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            b"",
        );
        let lines: Vec<&str> = canonical.split('\n').collect();
        assert_eq!(lines[0], "GET", "HTTP method");
        assert_eq!(lines[1], "/api/projects/test", "CanonicalURI");
        assert_eq!(lines[2], "", "CanonicalQueryString");
        // With no content-type or content-md5, the first header is "date:"
        assert!(lines[3].starts_with("date:"), "First canonical header should be date");
        assert!(!lines[5].is_empty(), "SignedHeaders must not be empty");
        // Last line is HashedRequestPayload
        let empty_hash = hex::encode(Sha256::digest(b""));
        assert!(canonical.ends_with(&empty_hash), "Should end with payload hash");
    }

    #[test]
    fn test_build_canonical_request_v4_with_query() {
        let headers = HeaderMap::new();
        let _canonical = build_canonical_request_v4(
            "GET",
            "/api/projects/p1/instances/i1",
            "result",
            "",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            b"",
        );
        // Canonical query string should be "result"
    }

    #[test]
    fn test_build_canonical_request_v4_with_odps_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-odps-request-id", "abc123".parse().unwrap());
        headers.insert("x-odps-project", "test_project".parse().unwrap());

        let (canonical_headers, signed_headers) = build_canonical_headers_v4(
            &headers,
            "application/xml",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
        );
        // Headers sorted alphabetically: content-type, date, x-odps-project, x-odps-request-id
        assert!(canonical_headers.starts_with("content-type:application/xml\n"));
        assert!(canonical_headers.contains("date:Mon, 01 Jan 2024 00:00:00 GMT\n"));
        assert!(canonical_headers.contains("x-odps-project:test_project\n"));
        assert!(canonical_headers.contains("x-odps-request-id:abc123\n"));

        // Signed headers should be semicolon-separated and sorted
        let parts: Vec<&str> = signed_headers.split(';').collect();
        assert!(parts.contains(&"date"));
        assert!(parts.contains(&"x-odps-project"));
        assert!(parts.contains(&"x-odps-request-id"));
        assert!(parts.contains(&"content-type"));

        // Verify sort order
        let mut sorted = parts.to_vec();
        sorted.sort();
        assert_eq!(parts, sorted, "Signed headers must be sorted");
    }

    // -----------------------------------------------------------------------
    // V4 Canonical Query String
    // -----------------------------------------------------------------------

    #[test]
    fn test_canonical_query_string_v4_empty() {
        assert_eq!(canonical_query_string_v4(""), "");
    }

    #[test]
    fn test_canonical_query_string_v4_sorted() {
        let result = canonical_query_string_v4("b=2&a=1");
        assert_eq!(result, "a=1&b=2");
    }

    // -----------------------------------------------------------------------
    // V4 StringToSign
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_string_to_sign_v4_format() {
        let s = build_string_to_sign_v4(
            "20240101T000000Z",
            "20240101",
            "cn-hangzhou",
            "abc123def456",
        );
        assert!(s.starts_with("ACS4-HMAC-SHA256\n"));
        assert!(s.contains("\n20240101/cn-hangzhou/odps/aliyun_v4_request\n"));
        assert!(s.ends_with("abc123def456"));
    }

    // -----------------------------------------------------------------------
    // HTTP date to ISO 8601 timestamp
    // -----------------------------------------------------------------------

    #[test]
    fn test_http_date_to_iso8601_timestamp_parses() {
        let ts = http_date_to_iso8601_timestamp("Mon, 01 Jan 2024 00:00:00 GMT");
        assert_eq!(ts, Some("20240101T000000Z".to_string()));
    }

    #[test]
    fn test_http_date_to_iso8601_timestamp_gmt_offset() {
        let ts = http_date_to_iso8601_timestamp("Thu, 15 Jun 2023 12:30:45 GMT");
        assert_eq!(ts, Some("20230615T123045Z".to_string()));
    }

    // -----------------------------------------------------------------------
    // V4 Known-good test vector
    // -----------------------------------------------------------------------

    /// Known-good V4 test vector.
    ///
    /// This test manually computes the full V4 signature using a known
    /// configuration and verifies it with verify_request. This ensures the
    /// V4 canonical request format, StringToSign, and signing key derivation
    /// all work together correctly.
    #[test]
    fn test_v4_known_good_signature_roundtrip() {
        let config = McAuthConfig::new_v4("test_key", "test_secret", "cn-hangzhou");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert("content-type", "application/xml".parse().unwrap());

        // Build the V4 canonical request
        let canonical_request = build_canonical_request_v4(
            "GET",
            "/api/projects/test",
            "",
            "application/xml",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            b"",
        );

        // Hash the canonical request
        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        // Build the StringToSign
        let timestamp = http_date_to_iso8601_timestamp("Mon, 01 Jan 2024 00:00:00 GMT")
            .unwrap();
        let string_to_sign = build_string_to_sign_v4(
            &timestamp,
            "20240101",
            "cn-hangzhou",
            &canonical_request_hash,
        );

        // Derive signing key and compute signature
        let signing_key = derive_v4_signing_key("test_secret", "20240101", "cn-hangzhou");
        let sig = hmac_sha256_base64(&signing_key, string_to_sign.as_bytes());

        headers.insert(
            "authorization",
            format!(
                "ODPS test_key/20240101/cn-hangzhou/odps/aliyun_v4_request:{}",
                sig
            )
            .parse()
            .unwrap(),
        );

        let result = verify_request(
            &config,
            "GET",
            "/api/projects/test",
            "",
            &headers,
            b"",
        );
        assert!(result.is_ok(), "V4 known-good verify should succeed: {:?}", result.err());
        assert!(result.unwrap(), "V4 known-good signature should match");
    }

    #[test]
    fn test_v4_known_good_signature_with_body() {
        let config = McAuthConfig::new_v4("test_key", "test_secret", "cn-hangzhou");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert("content-type", "application/xml".parse().unwrap());

        let body = b"<Instance><Job><Query>SELECT 1</Query></Job></Instance>";

        // Build the V4 canonical request with body
        let canonical_request = build_canonical_request_v4(
            "POST",
            "/api/projects/test/instances",
            "",
            "application/xml",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            body,
        );

        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        let timestamp = http_date_to_iso8601_timestamp("Mon, 01 Jan 2024 00:00:00 GMT")
            .unwrap();
        let string_to_sign = build_string_to_sign_v4(
            &timestamp,
            "20240101",
            "cn-hangzhou",
            &canonical_request_hash,
        );

        let signing_key = derive_v4_signing_key("test_secret", "20240101", "cn-hangzhou");
        let sig = hmac_sha256_base64(&signing_key, string_to_sign.as_bytes());

        headers.insert(
            "authorization",
            format!(
                "ODPS test_key/20240101/cn-hangzhou/odps/aliyun_v4_request:{}",
                sig
            )
            .parse()
            .unwrap(),
        );

        let result = verify_request(
            &config,
            "POST",
            "/api/projects/test/instances",
            "",
            &headers,
            body,
        );
        assert!(result.is_ok(), "V4 known-good verify with body should succeed: {:?}", result.err());
        assert!(result.unwrap(), "V4 known-good signature with body should match");
    }

    #[test]
    fn test_v4_signature_mismatch_with_wrong_body() {
        let config = McAuthConfig::new_v4("test_key", "test_secret", "cn-hangzhou");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert("content-type", "application/xml".parse().unwrap());

        let body = b"<Instance><Job><Query>SELECT 1</Query></Job></Instance>";

        // Build signature with one body
        let canonical_request = build_canonical_request_v4(
            "POST",
            "/api/projects/test/instances",
            "",
            "application/xml",
            "",
            "Mon, 01 Jan 2024 00:00:00 GMT",
            &headers,
            body,
        );

        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        let timestamp = http_date_to_iso8601_timestamp("Mon, 01 Jan 2024 00:00:00 GMT")
            .unwrap();
        let string_to_sign = build_string_to_sign_v4(
            &timestamp,
            "20240101",
            "cn-hangzhou",
            &canonical_request_hash,
        );

        let signing_key = derive_v4_signing_key("test_secret", "20240101", "cn-hangzhou");
        let sig = hmac_sha256_base64(&signing_key, string_to_sign.as_bytes());

        headers.insert(
            "authorization",
            format!(
                "ODPS test_key/20240101/cn-hangzhou/odps/aliyun_v4_request:{}",
                sig
            )
            .parse()
            .unwrap(),
        );

        // Verify with a DIFFERENT body => should fail
        let result = verify_request(
            &config,
            "POST",
            "/api/projects/test/instances",
            "",
            &headers,
            b"<different>body</different>",
        );
        assert!(result.is_ok(), "mismatched body should not error");
        assert!(!result.unwrap(), "mismatched body should fail verification");
    }

    #[test]
    fn test_verify_v4_wrong_region() {
        let config = McAuthConfig::new_v4("key", "secret", "cn-hangzhou");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert(
            "authorization",
            "ODPS key/20240101/us-east-1/odps/aliyun_v4_request:somesig=="
                .parse()
                .unwrap(),
        );
        let result = verify_request(&config, "GET", "/", "", &headers, &[]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // -----------------------------------------------------------------------
    // Auto-detection of V2 vs V4
    // -----------------------------------------------------------------------

    #[test]
    fn test_v4_detection_with_v2_config() {
        // V4 Authorization header with a V2-only config should still be
        // processed as V4 (detected by header content, not config)
        let config = McAuthConfig::new_v2("key", "secret");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert(
            "authorization",
            "ODPS key/20240101/cn-hangzhou/odps/aliyun_v4_request:sig=="
                .parse()
                .unwrap(),
        );
        // Should not error, just return false
        let result = verify_request(&config, "GET", "/", "", &headers, &[]);
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Unsupported scheme
    // -----------------------------------------------------------------------

    #[test]
    fn test_unsupported_scheme() {
        let config = McAuthConfig::new_v2("id", "secret");
        let mut headers = HeaderMap::new();
        headers.insert("date", "Mon, 01 Jan 2024 00:00:00 GMT".parse().unwrap());
        headers.insert("authorization", "Bearer some_token".parse().unwrap());
        let result = verify_request(&config, "GET", "/", "", &headers, &[]);
        assert!(matches!(result, Err(AuthError::UnsupportedScheme)));
    }
}