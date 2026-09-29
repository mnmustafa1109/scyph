# Changelog

All notable changes to the `scyph` workspace will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.3] - 2026-09-29

### Added
- **Initial Public Release** of Scyph as an opinionated, batteries-included modular backend library for Axum.
- **`scyph` (Facade Umbrella Crate)**:
  - Unified re-exports across all workspace components via feature flags (`auth`, `abac`, `cedar`, `db`, `query`, `health`, `notify`, `email`, `fcm`, `realtime`, `storage`, `s3`, `image`, `telemetry`, `ratelimit`, `utils`, `extractors`, `full`).
  - Standard framework `prelude` for single-line imports.
- **`scyph-core`**:
  - RFC 7807 compliant problem details error handling with `AppError` and `ErrorDetails`.
  - Standardized JSON envelope types: `ApiResponse<T>`, `PagedResponse<T>`, and `ResponseMeta`.
  - Foundational `Claims` and `Authorizable` identity traits.
- **`scyph-auth`**:
  - JWT creation, decoding, and validation using `jsonwebtoken` and `aws_lc_rs`.
  - Asynchronous and thread-pool Argon2id password hashing via `argon2`.
  - High-performance claims caching and token revocation using `moka`.
  - Axum `AuthUser<T>` and `OptionalAuthUser<T>` extractors with role-based route guards (`RoleRouterExt`).
- **`scyph-abac`**:
  - Attribute-Based Access Control policies with `AbacPolicy` trait.
  - Dynamic SQL filter building via `FilterBuilder`.
  - Optional Amazon Cedar policy engine evaluation (`cedar` feature).
- **`scyph-db`**:
  - PostgreSQL connection pool builder and lifecycle helpers via `sqlx`.
  - Safe transaction management (`begin`, `commit`, `rollback`).
  - Programmatic database migration runner and SQL seeding utilities.
  - Dynamic `QueryBuilder` extensions for search, filtering, sorting, and pagination (`query` feature).
- **`scyph-extractors`**:
  - Strongly typed, sanitized, and validated request payload extractors: `ValidatedJson<T>`, `SanitizedJson<T>`, `ValidatedQuery<T>`, and `ValidatedPath<T>` powered by `garde` and `sanitizer`.
- **`scyph-health`**:
  - Kubernetes liveness and readiness probe registries and HTTP handlers.
  - Seamless plug-in health extensions for database, storage, realtime, and notification backends.
- **`scyph-notify`**:
  - Asynchronous transactional email sending via `lettre` with `tera` HTML template rendering (`email` feature).
  - Firebase Cloud Messaging (FCM HTTP v1) push notifications with GCP service account token acquisition (`fcm` feature).
  - In-app notification repository traits.
- **`scyph-ratelimit`**:
  - IP-based and key-based sliding rate-limiting middleware powered by `tower-governor`.
  - Reverse-proxy smart client IP detection (`X-Forwarded-For`, `X-Real-IP`, `CF-Connecting-IP`).
- **`scyph-realtime`**:
  - Redis Pub/Sub WebSocket broadcaster for cross-replica message distribution and fanout.
  - Connection registry, bounded per-session client queues, and heartbeat ping/pong management.
- **`scyph-storage`**:
  - Storage provider abstractions via `StorageService`.
  - Production AWS S3 and MinIO (path-style aware) object store backend (`s3` feature).
  - Streaming multipart file extractors: `FileExtractor`, `MultiFileExtractor`, and `OptionalFileExtractor`.
  - Image thumbnail resizing and format conversion (`image` feature).
- **`scyph-telemetry`**:
  - Non-blocking structured tracing initialization via `tracing-subscriber` and `tracing-appender`.
  - Time-ordered UUIDv7 request ID tracking and propagation middleware (`x-request-id`).
  - Automatic response header injection (`x-response-time-ms`, `x-api-version`, `x-trace-id`) and JSON envelope metadata auto-injection.
  - Dynamic gzip/zstd/brotli HTTP response compression via `tower-http`.
- **`scyph-utils`**:
  - Constant-time HMAC-SHA256 webhook signature verification (direct digest and timestamped payloads).
  - Background worker loop runner with cancellation tokens and graceful shutdown joins (`spawn_worker`).
  - Base64 URL-safe cursor pagination primitives.
  - Distributed API idempotency key validation with Redis storage backend.
