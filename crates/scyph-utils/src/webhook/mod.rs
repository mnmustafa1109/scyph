//! HMAC-SHA256 signature verification and replay-attack prevention for inbound webhooks.
//!
//! This module supports two industry-standard webhook signature schemes:
//!
//! ## Timestamped Scheme (Stripe, Svix, Svix-compatible)
//!
//! The signature header contains both a Unix timestamp and a signature separated by commas:
//!
//! ```text
//! Stripe-Signature: t=1714000000,v1=abc123hex...
//! ```
//!
//! Verification steps:
//! 1. Extract `t=` timestamp and `v1=` signature from the header.
//! 2. Check that `|server_time - t|` is within `tolerance_secs` (replay protection).
//! 3. Compute `HMAC-SHA256("{t}.{body}", secret)` and compare in constant time.
//!
//! ## Direct Digest Scheme (GitHub, Shopify, generic)
//!
//! The signature header contains only the hex digest, often with a prefix:
//!
//! ```text
//! X-Hub-Signature-256: sha256=abc123hex...
//! ```
//!
//! Verification steps:
//! 1. Strip common prefixes (`sha256=`, `v1=`, `v0=`).
//! 2. Compute `HMAC-SHA256(body, secret)` and compare in constant time.
//! 3. No timestamp check (no timestamp in the header).
//!
//! ## Using the [`VerifiedWebhook`] Axum Extractor
//!
//! The [`VerifiedWebhook`] extractor automates all verification in a single Axum parameter:
//!
//! ```rust,ignore
//! use scyph_utils::webhook::{VerifiedWebhook, WebhookConfig};
//! use serde::Deserialize;
//!
//! pub struct StripeConfig;
//! impl WebhookConfig for StripeConfig {
//!     fn secret_env_var() -> &'static str { "STRIPE_WEBHOOK_SECRET" }
//!     fn header_name() -> &'static str { "Stripe-Signature" }
//! }
//!
//! #[derive(Deserialize)]
//! pub struct StripeEvent { pub id: String }
//!
//! async fn stripe_handler(
//!     VerifiedWebhook(event): VerifiedWebhook<StripeConfig, StripeEvent>,
//! ) {
//!     println!("Verified event: {}", event.id);
//! }
//! ```
//!
//! ## Using Low-Level Verifier Functions
//!
//! For providers not fitting into either scheme, use the low-level functions directly:
//!
//! ```rust
//! use scyph_utils::webhook::{verify_raw_webhook, verify_webhook, verify_webhook_header};
//! use chrono::Utc;
//!
//! // Auto-detect: timestamped or direct based on header content
//! let payload = b"{\"event\":\"ping\"}";
//! let header = "sha256=abc123"; // GitHub-style
//! // verify_webhook_header(payload, header, "secret", 300)?;
//!
//! // Explicit direct verification (no timestamp)
//! // verify_raw_webhook(payload, "sha256=abc123", "secret")?;
//! ```

pub mod error;
pub mod extractor;
pub mod verifier;

pub use error::WebhookError;
pub use extractor::{VerifiedWebhook, WebhookConfig};
pub use verifier::{verify_raw_webhook, verify_webhook, verify_webhook_header};
