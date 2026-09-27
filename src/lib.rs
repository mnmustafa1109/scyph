//! # Scyph
//!
//! `scyph` is a modular, high-performance framework for building production-grade web services with Axum.
//!
//! ## Framework Architecture
//!
//! `scyph` aggregates cross-cutting application concerns into decoupled, independently usable crates:
//! - **[`core`]**: Foundational RFC 7807 problem details ([`AppError`]), standard JSON response envelopes ([`ApiResponse`], [`PagedResponse`], [`ResponseMeta`]), and identity traits ([`Claims`]).
//! - **[`auth`]**: JWT token creation/verification, Argon2id password hashing, Moka claims caching, and role-based route guards.
//! - **[`abac`]**: Attribute-Based Access Control policies, SQL query [`scyph_abac::FilterBuilder`], and Cedar policy evaluation.
//! - **[`db`]**: PostgreSQL connection pooling via SQLx, transaction context management, and database migration/seeding tools.
//! - **[`health`]**: Service health registries and Kubernetes liveness/readiness endpoint handlers.
//! - **[`notify`]**: Email delivery via Lettre & Tera templates, Firebase Cloud Messaging push notifications, and in-app repository traits.
//! - **[`realtime`]**: Redis Pub/Sub WebSocket broadcasting, multi-recipient fanout, and typed real-time payload framing.
//! - **[`storage`]**: S3/MinIO and in-memory object storage abstractions with streaming multipart extractors.
//! - **[`telemetry`]**: Non-blocking tracing subscribers, time-ordered UUIDv7 request ID propagation, response metadata auto-injection, and dynamic compression.
//! - **[`ratelimit`]**: IP-based rate limiting layers built on `tower-governor`.
//! - **[`utils`]**: HMAC webhooks, background worker task loops, Base64 pagination cursors, and Redis API idempotency.
//!
//! ## Feature Flags
//!
//! - `auth`: Enables authentication and RBAC utilities (`scyph-auth`).
//! - `abac`: Enables Attribute-Based Access Control (`scyph-abac`).
//! - `cedar`: Enables Cedar policy evaluation in `scyph-abac`.
//! - `db`: Enables PostgreSQL pool management and migration tools (`scyph-db`).
//! - `health`: Enables health check endpoint registries (`scyph-health`).
//! - `notify`: Enables notification services (`scyph-notify`).
//! - `email`: Enables SMTP email delivery via Lettre in `scyph-notify`.
//! - `fcm`: Enables Firebase Cloud Messaging in `scyph-notify`.
//! - `realtime`: Enables Redis Pub/Sub WebSocket broadcaster (`scyph-realtime`).
//! - `storage`: Enables object storage and file extractors (`scyph-storage`).
//! - `s3`: Enables AWS S3 / MinIO backend in `scyph-storage`.
//! - `image`: Enables image thumbnail generation in `scyph-storage`.
//! - `telemetry`: Enables tracing, UUIDv7 request IDs, and auto-meta response injection (`scyph-telemetry`).
//! - `ratelimit`: Enables IP-based rate limiting middleware (`scyph-ratelimit`).
//! - `utils`: Enables utility primitives, HMAC webhooks, background workers, and API idempotency (`scyph-utils`).
//! - `extractors`: Enables type-safe validated JSON body, query parameter, and URL path extractors (`scyph-extractors`).
//! - `query`: Enables `QueryBuilder` dynamic filtering, sorting, pagination, and search extensions (`scyph-db`).
//! - `full`: Umbrella feature enabling all framework sub-crates and sub-features.
//!
//! ## Quickstart Example
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use scyph::prelude::*;

//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Initialize non-blocking telemetry logging
//!     let _guard = init_tracing().expect("Tracing subscriber initialized");
//!
//!     // 2. Define application router
//!     let app = Router::new()
//!         .route("/api/hello", get(hello_handler));
//!
//!     // 3. Wrap router with telemetry (tracing spans, UUIDv7 request IDs, auto-meta injection, compression)
//!     let app = with_telemetry(app);
//!
//!     Ok(())
//! }
//!
//! async fn hello_handler(RequestId(req_id): RequestId) -> ApiResponse<String> {
//!     ApiResponse::ok(format!("Hello! Request ID is {req_id}"))
//! }
//! ```

#![warn(missing_docs)]

/// Core foundational building blocks (RFC 7807 AppError, ApiResponse, Claims).
pub use scyph_core as core;

#[cfg(feature = "auth")]
/// Authentication and role-based access control.
pub use scyph_auth as auth;

#[cfg(feature = "abac")]
/// Attribute-based access control policies and Cedar policy engine integration.
pub use scyph_abac as abac;

#[cfg(feature = "db")]
/// Database connection pooling and transaction lifecycle helpers.
pub use scyph_db as db;

#[cfg(feature = "health")]
/// Kubernetes liveness and readiness health checks.
pub use scyph_health as health;

#[cfg(feature = "notify")]
/// Email, FCM push, and in-app notification services.
pub use scyph_notify as notify;

#[cfg(feature = "storage")]
/// Object storage abstractions, multipart extractors, and AWS S3 / MinIO service implementation.
pub use scyph_storage as storage;

#[cfg(feature = "realtime")]
/// Redis Pub/Sub WebSocket broadcaster for cross-replica message fanout.
pub use scyph_realtime as realtime;

#[cfg(feature = "telemetry")]
/// Structured tracing, non-blocking logging, UUIDv7 request ID middleware, response metadata auto-injection, and HTTP response compression.
pub use scyph_telemetry as telemetry;

#[cfg(feature = "ratelimit")]
/// IP-based and key-based rate limiting middleware integration via `tower-governor`.
pub use scyph_ratelimit as ratelimit;

#[cfg(feature = "utils")]
/// HMAC webhooks, background worker task loops, cursor pagination, and Redis API idempotency.
pub use scyph_utils as utils;

#[cfg(feature = "extractors")]
/// Type-safe validated JSON body, query parameter, and URL path extractors.
pub use scyph_extractors as extractors;

/// Convenient prelude re-exporting common framework types for single-line imports (`use scyph::prelude::*;`).
pub mod prelude {
    pub use scyph_core::{
        Action, ApiResponse, AppError, Authorizable, Claims, ErrorDetails, PagedResponse,
        ResponseMeta, Result,
    };

    #[cfg(feature = "auth")]
    pub use scyph_auth::{
        AuthCacheService, AuthError, AuthUser, OptionalAuthUser, PasswordService, RoleRouterExt,
        hash_password, hash_password_async, require_role, verify_password, verify_password_async,
    };

    #[cfg(feature = "abac")]
    pub use scyph_abac::{AbacError, AbacPolicy, AuthUserEnforceExt, FilterBuilder};

    #[cfg(feature = "db")]
    pub use scyph_db::{
        DbError, begin, build_pool, commit, rollback, run_migrations, run_migrations_from,
        run_seeds, run_seeds_from,
    };

    #[cfg(all(feature = "db", feature = "query"))]
    pub use scyph_db::{
        ApplyFiltering, ApplyPagination, ApplyRequestParams, ApplySearch, ApplySorting,
    };

    #[cfg(feature = "notify")]
    pub use scyph_notify::{
        EmailMessage, EmailService, EmailTemplate, NoEmailService, NoPushService, NotifyError,
        PushNotification, PushService, PushTemplate,
    };

    #[cfg(feature = "storage")]
    pub use scyph_storage::{
        ExtractedFile, FileConfig, FileExtractor, InMemoryStorageService, MultiFileExtractor,
        OptionalFileExtractor, StorageError, StorageService,
    };

    #[cfg(all(feature = "storage", feature = "s3"))]
    pub use scyph_storage::S3StorageService;

    #[cfg(all(feature = "storage", feature = "image"))]
    pub use scyph_storage::{
        StorageThumbnailExt, ThumbnailConfig, ThumbnailStoreResult, derive_thumbnail_key,
        derive_thumbnail_key_with_format, format_content_type, format_extension,
        generate_thumbnail, generate_thumbnail_with_format, thumbnail_key,
    };

    #[cfg(feature = "realtime")]
    pub use scyph_realtime::{
        ConnectionRegistry, RealtimeBroadcaster, RealtimeConfig, RealtimeError, RealtimeEvent,
        RealtimePayload, RealtimeSession,
    };

    #[cfg(feature = "telemetry")]
    pub use scyph_telemetry::{
        auto_meta_middleware, init_tracing, with_telemetry, with_telemetry_config,
        MakeRequestIdV7, RequestId, TelemetryConfig, TelemetryGuard,
    };

    #[cfg(feature = "ratelimit")]
    pub use scyph_ratelimit::{
        per_ip_layer, relaxed_layer, strict_layer, PeerRateLimitLayer, RateLimitConfig,
        RateLimitError, RateLimitRouterExt,
    };

    #[cfg(feature = "utils")]
    pub use scyph_utils::{
        Cursor, IdempotencyCheck, IdempotencyConfig, IdempotencyError, IdempotencyStore,
        PageParams, PaginationError, VerifiedWebhook, WebhookConfig, WebhookError, WorkerError,
        WorkerHandle, spawn_worker, spawn_worker_cancel, verify_webhook, verify_webhook_header,
    };

    #[cfg(feature = "extractors")]
    pub use scyph_extractors::{
        ExtractorError, FilterParams, LinkBuilder, LinkGenerator, PaginationParams, RequestParams,
        SanitizedJson, SearchParams, SortOrder, SortParams, TypedPath, ValidatedJson,
        ValidatedPath, ValidatedQuery, deserialize_number_from_string,
    };

    #[cfg(all(feature = "db", feature = "health"))]
    pub use scyph_db::DbHealthExt;

    #[cfg(all(feature = "realtime", feature = "health"))]
    pub use scyph_realtime::RealtimeHealthExt;

    #[cfg(all(feature = "storage", feature = "health"))]
    pub use scyph_storage::StorageHealthExt;

    #[cfg(feature = "health")]
    pub use scyph_health::{
        HealthFailure, HealthRegistry, IntoHealthResult, ServiceStatus, Status, health_routes,
    };
}

// ── Convenient Top-Level Re-exports ──────────────────────────────────────────
pub use scyph_core::{
    Action, ApiResponse, AppError, Authorizable, Claims, ErrorDetails, PagedResponse, ResponseMeta, Result,
};

#[cfg(feature = "auth")]
pub use scyph_auth::{AuthError, AuthUser, OptionalAuthUser};

#[cfg(feature = "abac")]
pub use scyph_abac::{AbacError, AbacPolicy, FilterBuilder};

#[cfg(feature = "db")]
pub use scyph_db::DbError;

#[cfg(all(feature = "db", feature = "query"))]
pub use scyph_db::{
    ApplyFiltering, ApplyPagination, ApplyRequestParams, ApplySearch, ApplySorting,
};

#[cfg(feature = "notify")]
pub use scyph_notify::{EmailTemplate, NoEmailService, NoPushService, NotifyError, PushTemplate};

#[cfg(feature = "storage")]
pub use scyph_storage::StorageError;

#[cfg(all(feature = "storage", feature = "image"))]
pub use scyph_storage::{
    StorageThumbnailExt, ThumbnailConfig, ThumbnailStoreResult, derive_thumbnail_key,
    derive_thumbnail_key_with_format, format_content_type, format_extension,
    generate_thumbnail, generate_thumbnail_with_format, thumbnail_key,
};

#[cfg(feature = "realtime")]
pub use scyph_realtime::{
    RealtimeBroadcaster, RealtimeError, RealtimeEvent, RealtimePayload, RealtimeSession,
};

#[cfg(feature = "telemetry")]
pub use scyph_telemetry::{
    auto_meta_middleware, init_tracing, with_telemetry, RequestId, TelemetryGuard,
};

#[cfg(feature = "ratelimit")]
pub use scyph_ratelimit::{
    per_ip_layer, relaxed_layer, strict_layer, PeerRateLimitLayer, RateLimitConfig, RateLimitError,
    RateLimitRouterExt,
};

#[cfg(feature = "utils")]
pub use scyph_utils::{
    IdempotencyError, PaginationError, VerifiedWebhook, WebhookConfig, WebhookError, WorkerError,
    WorkerHandle,
};

#[cfg(feature = "extractors")]
pub use scyph_extractors::{
    ExtractorError, FilterParams, LinkBuilder, LinkGenerator, PaginationParams, RequestParams,
    SanitizedJson, SearchParams, SortOrder, SortParams, TypedPath, ValidatedJson, ValidatedPath,
    ValidatedQuery, deserialize_number_from_string,
};

#[cfg(all(feature = "realtime", feature = "health"))]
pub use scyph_realtime::RealtimeHealthExt;
