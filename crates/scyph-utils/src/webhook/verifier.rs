//! HMAC-SHA256 webhook verifier logic.
//!
//! ## Two Verification Formats
//!
//! Different webhook providers use different signature header formats. This module supports both:
//!
//! ### 1. Timestamped Format (Stripe, Svix, Clerk)
//!
//! The header contains a Unix timestamp **and** the HMAC signature separated by commas:
//!
//! ```text
//! Stripe-Signature: t=1714000000,v1=abc123def456...
//! Svix-Signature: t=1714000000,v1=abc123def456...
//! ```
//!
//! The HMAC is computed over `"{timestamp}.{payload_bytes}"` — the timestamp is incorporated into
//! the signed message. This prevents **replay attacks**: an attacker who captures a valid webhook
//! cannot re-send it minutes later because the timestamp check (controlled by `tolerance_secs`) will
//! reject it. Use [`verify_webhook`] for this format.
//!
//! ### 2. Direct Digest Format (GitHub, Shopify, custom)
//!
//! The header contains only the HMAC signature (with an optional prefix):
//!
//! ```text
//! X-Hub-Signature-256: sha256=abc123def456...
//! X-Shopify-Hmac-Sha256: abc123def456...
//! ```
//!
//! The HMAC is computed directly over the raw payload bytes only — no timestamp component.
//! This format does **not** inherently protect against replay attacks, so providers using it
//! typically rely on other mechanisms (event IDs, short delivery windows). Use [`verify_raw_webhook`]
//! for this format.
//!
//! ### Automatic Detection via [`verify_webhook_header`]
//!
//! [`verify_webhook_header`] inspects the header value and automatically dispatches to the correct
//! verifier: if a `t=` timestamp component is found, it uses the timestamped format; otherwise it
//! falls back to direct digest verification. This is the recommended entry point for most use cases.

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
/// ## Direct Digest Format
///
/// This function handles the **direct digest** format where the header contains only the HMAC
/// signature with no timestamp component. The HMAC is computed as:
///
/// ```text
/// HMAC-SHA256(secret, payload_bytes)
/// ```
///
/// **Note**: This format does not protect against replay attacks on its own since there is no
/// timestamp in the signed message. Use this for providers like GitHub, Shopify, or any provider
/// that relies on HTTPS + short delivery windows for replay protection.
///
/// # Arguments
///
/// * `payload` - Raw binary body slice of the webhook request.
/// * `signature` - Expected hex signature string (may include `sha256=` or `v1=` prefix; stripped automatically).
/// * `secret` - Shared webhook signing secret.
///
/// # Errors
///
/// Returns [`WebhookError::SignatureMismatch`] if signature validation fails.
/// Returns [`WebhookError::HmacKey`] if the secret key is invalid.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::webhook::verify_raw_webhook;
/// use hmac::{Hmac, KeyInit, Mac};
/// use sha2::Sha256;
///
/// let payload = b"{\"action\":\"opened\",\"number\":1}";
/// let secret = "my_github_secret";
///
/// // Compute the expected signature as GitHub would
/// let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
/// mac.update(payload);
/// let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
///
/// // Both prefixed and raw hex forms are accepted
/// assert!(verify_raw_webhook(payload, &sig, secret).is_ok());
/// assert!(verify_raw_webhook(payload, sig.strip_prefix("sha256=").unwrap(), secret).is_ok());
/// ```
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

/// Helper function to parse standard webhook signature headers and dispatch to the correct verifier.
///
/// ## Automatic Format Detection
///
/// This function inspects the `header_val` string and routes to the appropriate verifier:
///
/// | Header contains `t=` or `,` | Routes to | Example providers |
/// |---|---|---|
/// | Yes | [`verify_webhook`] (timestamped, replay-safe) | Stripe, Svix, Clerk |
/// | No | [`verify_raw_webhook`] (direct digest) | GitHub, Shopify, custom |
///
/// ### Timestamped header example (Stripe):
/// ```text
/// Stripe-Signature: t=1714000000,v1=abc123...
/// ```
/// Parsed as: `timestamp = 1714000000`, `signature = "abc123..."`.
/// HMAC input: `"1714000000.{raw_body}"`.
///
/// ### Direct digest header example (GitHub):
/// ```text
/// X-Hub-Signature-256: sha256=abc123...
/// ```
/// Parsed as: `signature = "sha256=abc123..."` (prefix stripped automatically).
/// HMAC input: `raw_body`.
///
/// # Arguments
///
/// * `payload` - Raw binary body slice of the webhook request.
/// * `header_val` - Raw header value string (e.g. `"t=1714000000,v1=abc123..."` or `"sha256=abc123..."`).
/// * `secret` - Shared webhook signing secret.
/// * `tolerance_secs` - Maximum allowed age of the timestamp in seconds (only applies to timestamped format).
///
/// # Errors
///
/// Returns [`WebhookError::InvalidHeaderFormat`] if the header is malformed (missing signature component),
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
