//! Axum `FromRequest` extractor for type-safe, HMAC-verified webhooks.

use crate::webhook::verifier::verify_webhook_header;
use axum::{
    body::Bytes,
    extract::FromRequest,
    http::Request,
};
use serde::de::DeserializeOwned;
use std::{env, marker::PhantomData};

/// Declarative configuration template for webhook verification.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::webhook::WebhookConfig;
///
/// pub struct StripeWebhook;
///
/// impl WebhookConfig for StripeWebhook {
///     fn secret_env_var() -> &'static str { "STRIPE_WEBHOOK_SECRET" }
///     fn header_name() -> &'static str { "Stripe-Signature" }
/// }
/// ```
pub trait WebhookConfig {
    /// Environment variable name containing the shared webhook secret key.
    fn secret_env_var() -> &'static str;

    /// Request header name containing the signature (e.g. `"Stripe-Signature"` or `"X-Hub-Signature-256"`).
    fn header_name() -> &'static str;

    /// Maximum permitted age of the request timestamp in seconds (default: `300` / 5 minutes).
    fn tolerance_secs() -> i64 {
        300
    }
}

/// Type-safe Axum extractor for HMAC-SHA256 verified webhooks.
///
/// Automatically reads raw body bytes, verifies the signature header against `C::secret_env_var()`,
/// checks timestamp tolerance, and deserializes JSON into `T`.
///
/// # Type Parameters
/// - `C`: Your application's [`WebhookConfig`] implementation.
/// - `T`: Target JSON deserializable payload struct.
///
/// # Examples
///
/// ```rust,ignore
/// use axum::http::StatusCode;
/// use scyph_utils::webhook::{VerifiedWebhook, WebhookConfig};
/// use serde::Deserialize;
///
/// pub struct StripeWebhook;
/// impl WebhookConfig for StripeWebhook {
///     fn secret_env_var() -> &'static str { "STRIPE_WEBHOOK_SECRET" }
///     fn header_name() -> &'static str { "Stripe-Signature" }
/// }
///
/// #[derive(Deserialize)]
/// pub struct StripeEvent { pub id: String }
///
/// async fn webhook_handler(
///     VerifiedWebhook(event): VerifiedWebhook<StripeWebhook, StripeEvent>,
/// ) -> StatusCode {
///     println!("Verified event: {}", event.id);
///     StatusCode::OK
/// }
/// ```
#[derive(Debug, Clone)]
pub struct VerifiedWebhook<C: WebhookConfig, T: DeserializeOwned>(
    /// Parsed and verified payload struct.
    pub T,
    PhantomData<C>,
);

impl<C: WebhookConfig, T: DeserializeOwned> VerifiedWebhook<C, T> {
    /// Consumes the wrapper and returns the inner deserialized payload `T`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<C, T, S> FromRequest<S> for VerifiedWebhook<C, T>
where
    C: WebhookConfig + 'static,
    T: DeserializeOwned + 'static,
    S: Send + Sync,
{
    type Rejection = scyph_core::AppError;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers();

        let sig_header = headers
            .get(C::header_name())
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                scyph_core::AppError::BadRequest(format!(
                    "Missing expected webhook signature header '{}'",
                    C::header_name()
                ))
            })?
            .to_string();

        let secret = env::var(C::secret_env_var()).map_err(|_| {
            scyph_core::AppError::internal(format!(
                "Environment variable '{}' must be set for webhook verification",
                C::secret_env_var()
            ))
        })?;

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| scyph_core::AppError::BadRequest(e.to_string()))?;

        verify_webhook_header(&bytes, &sig_header, &secret, C::tolerance_secs())
            .map_err(scyph_core::AppError::from)?;

        let payload: T = serde_json::from_slice(&bytes)
            .map_err(|e| scyph_core::AppError::BadRequest(format!("Invalid webhook JSON payload: {e}")))?;

        Ok(Self(payload, PhantomData))
    }
}
