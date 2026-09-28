//! Axum `FromRequest` extractor for type-safe, HMAC-verified webhooks.
//!
//! This module provides the [`VerifiedWebhook`] extractor and the [`WebhookConfig`] trait for
//! declaratively configuring per-provider webhook verification.
//!
//! ## How It Works
//!
//! When `VerifiedWebhook<C, T>` is used as an Axum handler parameter, it performs the following
//! in a single extraction step:
//!
//! 1. Reads the raw request body bytes (consuming the body stream exactly once).
//! 2. Reads the configured signature header (`C::header_name()`).
//! 3. Reads the signing secret from the environment (`C::secret_env_var()`).
//! 4. Calls [`verify_webhook_header`] to validate the HMAC-SHA256 signature with timestamp tolerance.
//! 5. Deserializes the raw bytes as JSON into `T`.
//! 6. On any failure, returns an [`AppError`] HTTP 400/500 response.
//!
//! ## Registering Multiple Providers
//!
//! Define one [`WebhookConfig`] implementation per provider and use the same extractor pattern:
//!
//! ```rust,ignore
//! use scyph_utils::webhook::{VerifiedWebhook, WebhookConfig};
//! use serde::Deserialize;
//!
//! pub struct StripeWebhook;
//! impl WebhookConfig for StripeWebhook {
//!     fn secret_env_var() -> &'static str { "STRIPE_WEBHOOK_SECRET" }
//!     fn header_name() -> &'static str { "Stripe-Signature" }
//! }
//!
//! pub struct GithubWebhook;
//! impl WebhookConfig for GithubWebhook {
//!     fn secret_env_var() -> &'static str { "GITHUB_WEBHOOK_SECRET" }
//!     fn header_name() -> &'static str { "X-Hub-Signature-256" }
//! }
//!
//! #[derive(Deserialize)]
//! pub struct StripeEvent { pub id: String, pub r#type: String }
//!
//! #[derive(Deserialize)]
//! pub struct GithubPush { pub r#ref: String }
//!
//! async fn stripe_handler(VerifiedWebhook(event): VerifiedWebhook<StripeWebhook, StripeEvent>) { /* ... */ }
//! async fn github_handler(VerifiedWebhook(push): VerifiedWebhook<GithubWebhook, GithubPush>) { /* ... */ }
//! ```

use crate::webhook::verifier::verify_webhook_header;
use axum::{
    body::{Body, Bytes},
    extract::FromRequest,
    http::Request,
};
use scyph_core::AppError;
use serde::de::DeserializeOwned;
use std::{env, marker::PhantomData};

/// Declarative configuration template for webhook verification.
///
/// Implement this trait on a unit struct for each webhook provider your application integrates
/// with. The struct itself carries no data — it is a pure compile-time configuration marker used
/// to parameterize [`VerifiedWebhook<C, T>`].
///
/// ## Required Methods
///
/// - [`secret_env_var`](WebhookConfig::secret_env_var): Name of the environment variable holding
///   the shared webhook signing secret. This variable **must** be set at runtime.
/// - [`header_name`](WebhookConfig::header_name): HTTP header name where the provider sends the
///   signature (case-insensitive comparison is performed automatically by the HTTP layer).
///
/// ## Optional Override
///
/// - [`tolerance_secs`](WebhookConfig::tolerance_secs): Timestamp replay window in seconds.
///   Defaults to `300` (5 minutes). Only applies to providers using the timestamped format
///   (Stripe, Svix). Ignored for direct-digest providers (GitHub).
///
/// # Examples
///
/// ```rust
/// use scyph_utils::webhook::WebhookConfig;
///
/// pub struct StripeWebhook;
/// impl WebhookConfig for StripeWebhook {
///     fn secret_env_var() -> &'static str { "STRIPE_WEBHOOK_SECRET" }
///     fn header_name() -> &'static str { "Stripe-Signature" }
///     // Accept webhooks up to 10 minutes old (non-default)
///     fn tolerance_secs() -> i64 { 600 }
/// }
///
/// pub struct GithubWebhook;
/// impl WebhookConfig for GithubWebhook {
///     fn secret_env_var() -> &'static str { "GITHUB_WEBHOOK_SECRET" }
///     fn header_name() -> &'static str { "X-Hub-Signature-256" }
///     // tolerance_secs ignored for GitHub (direct digest format, no timestamp)
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
/// ## Type Parameters
///
/// - `C`: Your application's [`WebhookConfig`] implementation (unit struct, compile-time marker).
/// - `T`: Target JSON deserializable payload struct (must implement [`serde::de::DeserializeOwned`]).
///
/// ## Error Responses
///
/// | Condition | HTTP Status | Description |
/// |-----------|-------------|-------------|
/// | Missing signature header | 400 | `C::header_name()` not present |
/// | Missing `C::secret_env_var()` | 500 | Environment variable not set |
/// | Stale timestamp | 400 | Timestamp exceeds `tolerance_secs` |
/// | Signature mismatch | 400 | HMAC validation failed |
/// | Invalid JSON body | 400 | Deserialization into `T` failed |
///
/// ## Usage in Axum Router
///
/// ```rust,ignore
/// use axum::{Router, routing::post, http::StatusCode};
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
/// pub struct StripeEvent {
///     pub id: String,
///     pub r#type: String,
/// }
///
/// async fn stripe_webhook_handler(
///     VerifiedWebhook(event): VerifiedWebhook<StripeWebhook, StripeEvent>,
/// ) -> StatusCode {
///     println!("Received verified Stripe event: {} ({})", event.id, event.r#type);
///     StatusCode::OK
/// }
///
/// let router = Router::new()
///     .route("/webhooks/stripe", post(stripe_webhook_handler));
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
    type Rejection = AppError;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers();

        let sig_header = headers
            .get(C::header_name())
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "Missing expected webhook signature header '{}'",
                    C::header_name()
                ))
            })?
            .to_string();

        let secret = env::var(C::secret_env_var()).map_err(|_| {
            AppError::internal(format!(
                "Environment variable '{}' must be set for webhook verification",
                C::secret_env_var()
            ))
        })?;

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        verify_webhook_header(&bytes, &sig_header, &secret, C::tolerance_secs())
            .map_err(AppError::from)?;

        let payload: T = serde_json::from_slice(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid webhook JSON payload: {e}")))?;

        Ok(Self(payload, PhantomData))
    }
}
