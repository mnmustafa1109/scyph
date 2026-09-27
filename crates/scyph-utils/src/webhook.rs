// crates/scyph-utils/src/webhook.rs
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("Stale timestamp: delta {delta_secs}s exceeds tolerance {tolerance_secs}s")]
    StaleTimestamp {
        tolerance_secs: i64,
        delta_secs: i64,
    },
    #[error("Signature mismatch")]
    SignatureMismatch,
}

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
    let signed = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC key length");
    mac.update(signed.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if !ct_eq(signature.as_bytes(), expected.as_bytes()) {
        return Err(WebhookError::SignatureMismatch);
    }
    Ok(())
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
