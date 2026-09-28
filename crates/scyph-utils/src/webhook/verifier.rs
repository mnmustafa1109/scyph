//! HMAC-SHA256 webhook verifier logic.

use crate::webhook::error::WebhookError;
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Verifies an incoming HMAC-SHA256 webhook payload against a expected hex signature.
///
/// Ensures request timestamp is within the configured `tolerance_secs` window to prevent replay attacks,
/// and performs constant-time signature verification. Automatically strips common signature prefixes (`v1=`, `v0=`, `sha256=`).
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
/// Returns [`WebhookError::StaleTimestamp`] if timestamp exceeds tolerance,
/// or [`WebhookError::SignatureMismatch`] if signature validation fails.
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
) -> Result<(), WebhookError> {
    let delta = (Utc::now().timestamp() - timestamp).abs();
    if delta > tolerance_secs {
        return Err(WebhookError::StaleTimestamp {
            tolerance_secs,
            delta_secs: delta,
        });
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|e| WebhookError::HmacKey(e.to_string()))?;

    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);

    let expected = hex::encode(mac.finalize().into_bytes());

    let clean_sig = signature
        .strip_prefix("v1=")
        .or_else(|| signature.strip_prefix("v0="))
        .or_else(|| signature.strip_prefix("sha256="))
        .unwrap_or(signature);

    if !ct_eq(
        clean_sig.to_ascii_lowercase().as_bytes(),
        expected.as_bytes(),
    ) {
        return Err(WebhookError::SignatureMismatch);
    }
    Ok(())
}

/// Verifies an incoming HMAC-SHA256 webhook payload directly without timestamps (e.g. GitHub `X-Hub-Signature-256`, Shopify, etc.).
///
/// Automatically strips standard signature prefixes like `sha256=` or `v1=`.
///
/// # Arguments
///
/// * `payload` - Raw binary body slice of the webhook request.
/// * `signature` - Expected hex signature string.
/// * `secret` - Shared webhook signing secret.
///
/// # Errors
///
/// Returns [`WebhookError::SignatureMismatch`] if signature validation fails.
pub fn verify_raw_webhook(
    payload: &[u8],
    signature: &str,
    secret: &str,
) -> Result<(), WebhookError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|e| WebhookError::HmacKey(e.to_string()))?;

    mac.update(payload);
    let expected = hex::encode(mac.finalize().into_bytes());

    let clean_sig = signature
        .strip_prefix("sha256=")
        .or_else(|| signature.strip_prefix("v1="))
        .or_else(|| signature.strip_prefix("v0="))
        .unwrap_or(signature);

    if !ct_eq(
        clean_sig.to_ascii_lowercase().as_bytes(),
        expected.as_bytes(),
    ) {
        return Err(WebhookError::SignatureMismatch);
    }
    Ok(())
}

/// Helper function to parse standard webhook signature headers.
///
/// Supports:
/// - Timestamped headers (e.g. Stripe `t=1234567,v1=hex_signature`)
/// - Direct digest headers (e.g. GitHub `sha256=hex_signature` or raw hex)
///
/// # Errors
///
/// Returns [`WebhookError::InvalidHeaderFormat`] if the header is malformed,
/// or verification errors from [`verify_webhook`] / [`verify_raw_webhook`].
pub fn verify_webhook_header(
    payload: &[u8],
    header_val: &str,
    secret: &str,
    tolerance_secs: i64,
) -> Result<(), WebhookError> {
    let mut timestamp: Option<i64> = None;
    let mut signature: Option<&str> = None;

    if header_val.contains(',') || header_val.contains("t=") {
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
    } else {
        // Direct signature header (e.g. GitHub `sha256=...` or raw hex)
        signature = Some(header_val.trim());
    }

    if let (Some(ts), Some(sig)) = (timestamp, signature) {
        verify_webhook(payload, sig, secret, ts, tolerance_secs)
    } else if let Some(sig) = signature {
        // Fall back to direct raw payload HMAC verification when no timestamp component is present
        verify_raw_webhook(payload, sig, secret)
    } else {
        Err(WebhookError::InvalidHeaderFormat(
            "Missing signature component in webhook header".to_string(),
        ))
    }
}

/// Constant-time byte slice comparison to prevent timing attacks.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
