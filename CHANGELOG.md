# Changelog

All notable changes to the `scyph` workspace will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Fixed
- **`scyph-auth`**: `RoleRouterExt::require_roles` added its layers in the wrong order, so the role-check middleware could never see the `AllowedRoles` extension it depends on — every protected route failed with HTTP 500 (`MissingExtension` failure), instead of returning 200 / 403 / 401 as documented. The `Extension` layer is now applied last (outermost) and regression tests pin the behavior for shipped-consuming apps on 0.1.4.
- **`scyph-storage`**: S3 health probes (`StorageHealthExt`) classified every failure as transient — the SDK's error string never contains `Forbidden`/`NoSuchBucket` (it prints `service error`), so wrong credentials or a missing bucket showed up as `Status::Sick` ("may recover") instead of `Status::Deceased` (needs operator action). `S3StorageService::test_connection()` now surfaces the failure as a typed error (HTTP status **and** S3 error code), so probes and the boot check classify it the same way (see the `Changed` entries below); the `from_env()` boot check reuses the same call instead of duplicating the ping. Wrong credentials and a missing bucket are now `Deceased`; a reachable bucket whose policy lacks `s3:ListBucket` stays `Healthy`.
- **`scyph-storage`**: S3 clients were built with the SDK default timeout config (connect timeout only), so an endpoint that accepted TCP and then stopped answering (paused MinIO, broken proxy, LB misroute) hung health probes **and uploads forever** — one wedged probe stalled the caller's sequential probe loop, leaving `/readyz` reporting the boot-time statuses while dependencies were actually down. Clients now set a connect timeout (3s), a per-read timeout (30s), and an overall operation timeout (300s), so a wedged or trickling endpoint costs one tick and a `Status::Sick` instead of freezing readiness.
- **`scyph-notify`**: SMTP health probes were unbounded except by lettre's 60s per-command timeout, and a fresh connection costs connect + EHLO (+ STARTTLS + EHLO + AUTH) — a relay that accepted TCP and stalled mid-handshake could freeze the caller's sequential probe loop for minutes, hiding other dependencies' outages from `/readyz`. Each probe is now bounded at 5s (a stalled relay reports `Status::Sick` after one tick), and the boot handshake in `LettreSMTPService::from_env()` gets the same 5s bound, so a stalled relay fails startup instead of holding it for a minute. The FCM probe gets the same bound on both of its steps — the OAuth token fetch (`gcp_auth` sets no timeout) and the validate-only send (`Client::new()` has no default timeout in reqwest 0.13) — so a stalled Google endpoint no longer freezes the probe loop.
- **`scyph-db`**: the Postgres probe (`SELECT 1` in `DbHealthExt`) was bounded only by the pool's connect/acquire window, so a server that accepted TCP and stopped answering could stall the caller's sequential probe loop instead of reporting a failure. It now carries the same 5s bound as the S3 and SMTP probes and reports `Status::Sick` ("may recover") after one tick.
- **`scyph-storage`**: dropped the `sigv4a` feature from `aws-sdk-s3` (and transitively `aws-runtime`/`aws-sigv4`), which was the only path pulling in `crypto-bigint 0.5.5` (RUSTSEC-2026-0387, timing side-channel in constant-time arithmetic). No code in the crate uses SigV4a (multi-region access points), so removing it eliminates the advisory with no behavior change; `cargo deny` advisories now pass.

### Changed
- **Breaking — `scyph-notify`**: `LettreSMTPService::from_env()` now reads credentials from `SMTP_USER` / `SMTP_PASS` (was `SMTP_USERNAME` / `SMTP_PASSWORD`) and the sender from `FROM_EMAIL`, optionally paired with `FROM_NAME` (rendered as `"Name <email>"`), instead of the combined `SMTP_FROM`. Deployments still using the old names fail at boot with a `NotifyError::Configuration` message naming the new variable — the break is loud at startup, not a silent email drop. Update the environment (`.env.example` reflects the new names) rather than the code.
- **Breaking — behavior — `scyph-notify`**: `TemplateEngine::from_glob()` (and the `from_env()`/`from_dir()` wrappers) now return `NotifyError::Template` when the glob matched **zero** templates. Tera walks a missing directory to an empty set without error, so a wrong `TEMPLATES_DIR` or an unshipped `templates/` directory used to boot an engine that failed every render at request time — emails silently never arrived while API calls reported success.
- **Breaking — `scyph-storage`**: `S3StorageService::from_env()` now performs a 5s `test_connection()` ping before returning, so a dead endpoint or bad/missing AWS credentials fails at boot (`StorageError::Configuration`) instead of on the first upload — mirroring `LettreSMTPService::from_env()`, which already handshakes at boot. The client is no longer handed out unvalidated. The ping uses `ListObjectsV2` (`max-keys=1`, same `s3:ListBucket` IAM permission, same LIST billing class as `HeadBucket`) because a HEAD response has an empty body — with `ListObjectsV2` the answer carries an S3 error `<Code>` that the boot check and the health probe classify the same way: a 403 whose body says `AccessDenied` (a policy without `s3:ListBucket`) is treated as reachable (warning only, startup continues), while a 403 naming bad credentials (`InvalidAccessKeyId`, `SignatureDoesNotMatch`, `ExpiredToken`, `InvalidClientTokenId`) fails startup with `StorageError::Configuration`. The timeout and boot-check errors now name the endpoint that was contacted.
- **Breaking — behavior — `scyph-storage`**: `StorageService::test_connection()` now classifies failures by variant instead of embedding ` (HTTP 403)` in a `StorageError::S3` message: a 403 whose body says `AccessDenied` (or has no parseable body) returns the new `StorageError::Forbidden` (**breaking** for code matching `StorageError` exhaustively), a 403 naming bad credentials returns `StorageError::Configuration`, and a 404 (or `NoSuchBucket`) returns `StorageError::NotFound`. `StorageHealthExt` treats `Forbidden` as **reachable** (logged warning, `Status::Healthy`), matching `from_env()`'s boot decision to accept a least-privilege policy without `s3:ListBucket`: `HealthRegistry::is_ready()` rejects any non-`Healthy` required component (`Sick` and `Recovering` included), so anything else pins `/readyz` at 503 forever for a deployment that boots and uploads fine. `Configuration` (wrong credentials) and `NotFound` (missing bucket) are **fatal** (`Status::Deceased`) — no probe fixes a typo, and reporting `Healthy` while every upload is rejected would hide it.
- **`scyph-notify`**: the workspace `lettre` dependency now enables the `pool` feature, so SMTP health probes reuse a pooled connection (NOOP) instead of opening a fresh TCP + STARTTLS + AUTH handshake and sending QUIT on every probe tick — previously thousands of authenticated handshakes per day against the relay from each process. Connections idle out after lettre's default 60s `idle_timeout`, so a probe tick slower than that still re-handshakes; at the documented ~15s cadence the pool does reuse the connection.
- **Breaking — `scyph-abac` (cedar feature)**: `CedarSchemaBuilder::build_json_schema()` is renamed to `build()` and now emits the human-readable Cedar schema DSL (compatible with [`cedar_policy::Schema`]), not JSON. The old JSON output never parsed correctly under cedar-policy 4.x (`resourceTypes: null` is invalid). Policies are now statically validated against the provided schema at `CedarAuthorizer::from_str` construction via `Validator` in strict mode — invalid policies now fail at startup with `CedarError::PolicyValidation` instead of denying requests silently at runtime. Schema input accepts both Cedar DSL and JSON (`from_json_str` fallback). The builder's `Boolean` variant is renamed `CedarType::Bool` (emitting `Boolean` in the DSL made schemas unparseable under cedar-policy 4.x). New `CedarSchemaBuilder::for_principal()` sets a principal type other than `User`. New `add_action` method registers custom actions without duplicating built-in CRUD.

### Added
- **`scyph-auth`**: optional `JWT_ISSUER` — when set, `create_token` stamps every token with that value as its `iss` claim and `verify_token` requires a matching `iss` (plus the claim being present). Unset, behavior is unchanged: tokens carry no `iss` and verification is issuer-agnostic, so rolling the variable out invalidates outstanding tokens (holders re-authenticate) rather than breaking anything at compile time. Covered by `jwt_issuer_test`.
- **`scyph-notify`**: `REPLY_TO_EMAIL` (optional) — `LettreSMTPService` now sets the `Reply-To` header on outgoing mail when the variable is set; unset behaves exactly as before (no header). Invalid addresses fail `from_env()` at boot with a `Configuration` error.
- **`scyph-db`**: `is_unique_violation(err: &sqlx::Error) -> bool` helper and `DbError::is_unique_violation(&self) -> bool` method to detect PostgreSQL unique-constraint violations (SQLSTATE `23505`) for mapping duplicate keys to `409 Conflict`.
- **`scyph-storage`**: `delete_with_retry(storage, key)` fire-and-forget background deletion with a single retry after 1 s, plus `scyph_storage::background` module.
- **`scyph-auth`**: `jwt_secret_from_env()` reads `JWT_SECRET` and refuses secrets shorter than 32 bytes (256 bits of entropy) with the new `JwtError::Config` variant (**breaking** for downstream code that `match`es `JwtError` exhaustively), so applications fail fast at boot instead of signing brute-forceable (forgeable) tokens. `.env.example`, the `scyph_auth` quickstart and the `jwt` module docs now point at it as the way to read the secret.
- **`scyph-core`**: `ApiResponse` now carries an extra `headers: HeaderMap` field (skipped in JSON) applied to the Axum response on conversion, plus `with_header(name, value)` and `with_location(location)` builders — handy for `201 Created` responses that need a `Location` header. Parse failures in `with_location` are ignored.
- **`scyph-notify`**: `FcmPushService::base_url()` exposes the FCM API base URL the service posts to (defaults to `https://fcm.googleapis.com`); `send_fcm_request` now takes the base URL as a parameter instead of hardcoding it, so the send path can be pointed at a fake endpoint.
- `ResponseMeta::library_version` and the `x-library-version` response header now report the Scyph library version alongside the application's version.
- `TelemetryConfig::library_version` exposes the reported library version (defaults to the Scyph version).
- **`scyph-auth`**: `ARGON2_SECRET` is now actually wired in — `PasswordService::from_env()` reads it as the Argon2 pepper (PHC "secret key") applied to both hashing and verification; `PasswordService::with_secret` sets it programmatically.
- **`scyph-auth`**: New `password_test` coverage for pepper semantics (matching pepper verifies; different/absent pepper rejects).
- **`scyph-utils`**: `IdempotencyConfig::in_progress_ttl` / `IDEMPOTENCY_IN_PROGRESS_TTL_SECS` (default 60s) — the `IN_PROGRESS` reservation now self-heals shortly after a crash instead of blocking retries for the full result TTL. **`complete` is atomic** (Redis Lua): a late-retry execution can no longer overwrite the first completer's cached response — it gets `IdempotencyError::Conflict` instead. `cancel` is also atomic and only removes a still-in-progress reservation — a failed late retry can never delete the first completer's cached response. Known ceiling: a handler running *longer* than `in_progress_ttl` can still be executed twice concurrently (its computed result is then discarded at completion); keep handlers within the reservation window.
- **`scyph-health`**: `health_routes_verbose` exposes unmasked `/readyz` details for internal-only ports.

### Changed
- `TelemetryConfig::api_version` is documented as the application (backend) version; applications should set it with `with_api_version` instead of inheriting the Scyph version.
- **Security — dependencies**: the AWS SDK now compiles with `default-https-client` only (hyper 1.x + rustls 0.23.46 / rustls-webpki 0.103.16). The legacy HTTPS stack (`RUSTSEC-2026-0346`, `RUSTSEC-2026-0349`, `RUSTSEC-2026-0382` via rustls 0.21 / rustls-webpki 0.101) is no longer compiled or present in `Cargo.lock`. S3-over-TLS behavior is unchanged for `BehaviorVersion::latest()` users.
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
