//! HMAC-SHA256 signature verification and replay attack prevention for webhooks.

use crate::error::UtilsError;
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Verifies an incoming HMAC-SHA256 webhook payload against a expected hex signature.
///
/// Ensures request timestamp is within the configured `tolerance_secs` window to prevent replay attacks,
/// and performs constant-time signature verification.
///
/// # Arguments
///
/// * `payload` - Raw binary body slice of the webhook request.
/// * `signature` - Expected hex signature string (e.g. from header).
/// * `secret` - Shared webhook signing secret.
/// * `timestamp` - Unix timestamp in seconds from the webhook header.
/// * `tolerance_secs` - Maximum allowed age of the timestamp in seconds.
///
/// # Errors
///
/// Returns [`UtilsError::StaleTimestamp`] if timestamp exceeds tolerance,
/// or [`UtilsError::SignatureMismatch`] if signature validation fails.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::webhook::verify_webhook;
/// use chrono::Utc;
///
/// let payload = b"{\"event\":\"user.created\"}";
/// let secret = "whsec_my_secret_key";
/// let timestamp = Utc::now().timestamp();
///
/// // Compute valid signature for example test
/// use hmac::{Hmac, KeyInit, Mac};
/// use sha2::Sha256;
/// let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
/// mac.update(format!("{timestamp}.").as_bytes());
/// mac.update(payload);
/// let sig = hex::encode(mac.finalize().into_bytes());
///
/// assert!(verify_webhook(payload, &sig, secret, timestamp, 300).is_ok());
/// ```
pub fn verify_webhook(
    payload: &[u8],
    signature: &str,
    secret: &str,
    timestamp: i64,
    tolerance_secs: i64,
) -> Result<(), UtilsError> {
    let delta = (Utc::now().timestamp() - timestamp).abs();
    if delta > tolerance_secs {
        return Err(UtilsError::StaleTimestamp {
            tolerance_secs,
            delta_secs: delta,
        });
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|e| UtilsError::HmacKey(e.to_string()))?;

    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);

    let expected = hex::encode(mac.finalize().into_bytes());

    if !ct_eq(signature.as_bytes(), expected.as_bytes()) {
        return Err(UtilsError::SignatureMismatch);
    }
    Ok(())
}

/// Helper function to parse standard `t=1234567,v1=hex_signature` webhook headers.
///
/// Extracted from standard webhook providers (Stripe, GitHub, OpenWeave).
///
/// # Errors
///
/// Returns [`UtilsError::InvalidHeaderFormat`] if `header_val` is malformed,
/// or errors from [`verify_webhook`].
pub fn verify_webhook_header(
    payload: &[u8],
    header_val: &str,
    secret: &str,
    tolerance_secs: i64,
) -> Result<(), UtilsError> {
    let mut timestamp: Option<i64> = None;
    let mut signature: Option<&str> = None;

    for part in header_val.split(',') {
        let mut kv = part.splitn(2, '=');
        let k = kv.next().unwrap_or("").trim();
        let v = kv.next().unwrap_or("").trim();

        match k {
            "t" => {
                timestamp = v.parse::<i64>().ok();
            }
            "v1" | "v0" | "sig" => {
                signature = Some(v);
            }
            _ => {}
        }
    }

    let ts = timestamp.ok_or_else(|| {
        UtilsError::InvalidHeaderFormat("Missing timestamp 't' component".to_string())
    })?;

    let sig = signature.ok_or_else(|| {
        UtilsError::InvalidHeaderFormat("Missing signature 'v1' component".to_string())
    })?;

    verify_webhook(payload, sig, secret, ts, tolerance_secs)
}

/// Constant-time byte slice comparison to prevent timing attacks.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
