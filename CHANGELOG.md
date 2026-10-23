# Changelog

All notable changes to the `scyph` workspace will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- `ResponseMeta::library_version` and the `x-library-version` response header now report the Scyph library version alongside the application's version.
- `TelemetryConfig::library_version` exposes the reported library version (defaults to the Scyph version).
- **`scyph-auth`**: `ARGON2_SECRET` is now actually wired in — `PasswordService::from_env()` reads it as the Argon2 pepper (PHC "secret key") applied to both hashing and verification; `PasswordService::with_secret` sets it programmatically.
- **`scyph-auth`**: New `password_test` coverage for pepper semantics (matching pepper verifies; different/absent pepper rejects).
- **`scyph-utils`**: `IdempotencyConfig::in_progress_ttl` / `IDEMPOTENCY_IN_PROGRESS_TTL_SECS` (default 60s) — the `IN_PROGRESS` reservation now self-heals shortly after a crash instead of blocking retries for the full result TTL. **`complete` is atomic** (Redis Lua): a late-retry execution can no longer overwrite the first completer's cached response — it gets `IdempotencyError::Conflict` instead. `cancel` is also atomic and only removes a still-in-progress reservation — a failed late retry can never delete the first completer's cached response. Known ceiling: a handler running *longer* than `in_progress_ttl` can still be executed twice concurrently (its computed result is then discarded at completion); keep handlers within the reservation window.
- **`scyph-health`**: `health_routes_verbose` exposes unmasked `/readyz` details for internal-only ports.

### Changed
- `TelemetryConfig::api_version` is documented as the application (backend) version; applications should set it with `with_api_version` instead of inheriting the Scyph version.
- **Breaking — `scyph-health`**: `health_routes` now **masks** the `details` field of every component in the `/readyz` body (generic marker replaces error strings) to avoid leaking infrastructure internals from publicly exposed probes. Use the new `health_routes_verbose` on internal-only ports when full details are needed.
- **Breaking — behavior — `scyph-auth`**: hashes produced via `PasswordService::hash_password` when a pepper is configured (previously `ARGON2_SECRET` was documented but ignored); verification now also applies the pepper. Existing unpeppered hashes still verify with the default (no-pepper) service.
- **Breaking — telemetry**: auto-meta response injection now caps re-scanned JSON bodies at **256 KiB** (previously 2 MiB). Only responses whose size is known (declared `Content-Length` / exact body size hint) and within the cap get meta injected; streams/chunked bodies and oversize responses keep their telemetry headers and pass through with the body untouched (no meta injection), reducing latency and memory churn — responses are never replaced with an empty body anymore.
- **`scyph-storage`**: `ThumbnailConfig::generate`, `ExtractedFile::thumbnail`, `generate_thumbnail(_with_format)` are now `async` and run image decode/resize on `tokio::task::spawn_blocking`; decoding is bounded by a 64 MiB allocation limit (decompression-bomb guard). The raw-image parameter took `&[u8]` before and now takes `impl Into<bytes::Bytes>` — pass `Vec<u8>` or `Bytes` instead of a borrowed runtime slice (static literals keep working).
- **`scyph-realtime`**: `publish_to_many` / `publish_payload_to_many` publish to all recipients concurrently in chunks of 256 over a cloned multiplexed connection instead of sequentially.
- **`scyph-utils`**: `IdempotencyStore` reuses a single cached multiplexed Redis connection (with automatic reconnect on failure) instead of opening a connection per request.
- **`scyph-utils`**: Webhook signing secrets (`C::secret_env_var()`) are read once per provider and cached process-wide instead of per request.
- **`scyph-extractors`**: `LinkGenerator` reads `APP_BASE_URL` / `ALLOWED_HOSTS` once from a process-wide cache instead of per request.
- **`scyph-db`**: Removed duplicated migration/seed cascade logic into an internal shared helper; behavior unchanged.
- **`.env.example`**: Removed dead `SCYPH_MAX_PAGE_SIZE` / `SCYPH_DEFAULT_PAGE_SIZE` variables (pagination limits are compiled-in constants: default 20, cap 100).
- **`scyph-ratelimit`**: Documentation now states explicitly that `smart` layers trust client-supplied IP headers and are safe only behind proxies that unconditionally overwrite them (behavior unchanged).

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
