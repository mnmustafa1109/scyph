//! # Scyph
//!
//! `scyph` is a modular framework for building production-grade web services with Axum.
//!
//! ## Feature Flags
//!
//! - `auth`: Enables authentication (JWT, Argon2id, Moka caching, RBAC).
//! - `abac`: Enables Attribute-Based Access Control (SQL FilterBuilder, Cedar engine).
//! - `db`: Enables PostgreSQL pool management, transaction helpers, and migrations.

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

/// Convenient prelude re-exporting common framework types for single-line imports (`use scyph::prelude::*;`).
pub mod prelude {
    pub use scyph_core::{
        Action, ApiResponse, AppError, Authorizable, Claims, PagedResponse, Result,
    };

    #[cfg(feature = "auth")]
    pub use scyph_auth::{
        AuthCacheService, AuthError, AuthUser, OptionalAuthUser, RoleRouterExt, hash_password,
        hash_password_async, require_role, verify_password, verify_password_async,
    };

    #[cfg(feature = "abac")]
    pub use scyph_abac::{AbacError, AbacPolicy, AuthUserEnforceExt, FilterBuilder};

    #[cfg(feature = "db")]
    pub use scyph_db::{
        DbError, begin, build_pool, commit, rollback, run_migrations, run_migrations_from,
        run_seeds, run_seeds_from,
    };

    #[cfg(feature = "notify")]
    pub use scyph_notify::{
        EmailMessage, EmailService, EmailTemplate, NoEmailService, NoPushService, NotifyError,
        PushNotification, PushService, PushTemplate,
    };

    #[cfg(all(feature = "db", feature = "health"))]
    pub use scyph_db::DbHealthExt;

    #[cfg(feature = "health")]
    pub use scyph_health::{
        HealthFailure, HealthRegistry, IntoHealthResult, ServiceStatus, Status, health_routes,
    };
}

// ── Convenient Top-Level Re-exports ──────────────────────────────────────────
pub use scyph_core::{Action, ApiResponse, AppError, Authorizable, Claims, PagedResponse, Result};

#[cfg(feature = "auth")]
pub use scyph_auth::{AuthError, AuthUser, OptionalAuthUser};

#[cfg(feature = "abac")]
pub use scyph_abac::{AbacError, AbacPolicy, FilterBuilder};

#[cfg(feature = "db")]
pub use scyph_db::DbError;

#[cfg(feature = "notify")]
pub use scyph_notify::{EmailTemplate, NoEmailService, NoPushService, NotifyError, PushTemplate};
