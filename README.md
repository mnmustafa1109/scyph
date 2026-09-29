<div align="center">

# ⚡ Scyph

**An opinionated, batteries-included backend library for building production-grade web services with Axum.**

[![Crates.io](https://img.shields.io/crates/v/scyph.svg?style=flat-square&color=blue)](https://crates.io/crates/scyph)
[![Documentation](https://img.shields.io/docsrs/scyph?style=flat-square&label=docs.rs)](https://docs.rs/scyph)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](LICENSE-MIT)
[![Rust 1.98+](https://img.shields.io/badge/rust-1.98%2B-orange.svg?style=flat-square)](https://www.rust-lang.org)

</div>

---

> ⚠️ **Project Status: Alpha (`v0.1.x`)**  
> `scyph` is currently in active **Alpha**. It is battle-tested daily in **two production codebases**. We are gathering real-world performance metrics and telemetry, and plan to enter **Beta in approximately 6 months**. APIs may evolve slightly during this period.

---

## 💡 What is Scyph? (And What It Isn't)

**Scyph is a library, NOT a framework.**

In standard backend frameworks (like Loco, Poem, or Actix-web wrappers), the framework controls your application's lifecycle: it owns `main()`, generates hidden boilerplate through heavy macros, dictates project directory layouts, and forces you into rigid architectural constraints (Inversion of Control).

**Scyph takes the opposite approach.** You retain complete control over:
- Your `main()` entrypoint
- Your Tokio async runtime
- Your standard [`axum::Router`](https://docs.rs/axum/latest/axum/struct.Router.html)
- Your domain logic, data models, and folder layout

Instead of hijacking your code, **Scyph provides opinionated, battle-tested, modular building blocks** that snap directly into native Axum routers and extractors.

---

## 🎯 Why Was Scyph Created?

Every time a new backend service needed to be launched, the story was always the same:
> **An entire week was burned just setting up boilerplate.**

Writing the same JWT auth validation, hashing passwords with Argon2, structuring RFC 7807 problem details error envelopes, configuring SQLx connection pools and transaction helpers, wiring up non-blocking tracing with request ID propagation, writing multipart S3 streaming extractors, setting up Redis rate limiters, webhooks, and Kubernetes health checks... over and over again.

While existing Rust web frameworks existed, none of them felt right:
1. **They felt too magical or intrusive:** They hid essential request lifecycles behind macro-heavy abstractions or code generators.
2. **We couldn't agree with their design philosophies:** Many enforced monolithic paradigms, rigid ORMs, or opinionated directory structures that got in the way when microservices or custom data layers were needed.
3. **Fragile dependencies & C-binding issues:** Many frameworks or libraries relied on native OpenSSL or complex C toolchains rather than pure-Rust, memory-safe TLS and crypto primitives.

**Scyph was built to eliminate that boilerplate week permanently**, giving you a production-hardened foundation in minutes while keeping your codebase idiomatic, explicit, and lightweight.

---

## 🚀 How Scyph Differentiates Itself

| Feature | Monolithic Frameworks | Bare Axum | **Scyph** |
| :--- | :--- | :--- | :--- |
| **Control Model** | Inversion of Control (Framework owns `main`) | You own `main` | **You own `main` & `Router` (Pure Library)** |
| **Boilerplate** | Low, but rigid & hidden | High (days to weeks of repetitive plumbing) | **Near zero boilerplate, fully explicit** |
| **Modularity** | Monolithic lock-in | Assemble everything from scratch | **Independent sub-crates + umbrella facade** |
| **API Standards** | Ad-hoc responses & error formats | DIY response types | **Built-in RFC 7807 Problem Details & Envelopes** |
| **Telemetry & IDs** | Manual tracing wiring | Manual setup | **Zero-config non-blocking tracing + UUIDv7 IDs** |
| **Security & TLS** | Often defaults to OpenSSL | Depends on crates chosen | **Pure-Rust memory-safe TLS (`rustls` / `aws-lc-rs`)** |
| **Database Ergonomics** | Heavy ORMs with high compile times | Raw SQLx boilerplate | **SQLx pooling + dynamic search/sort/filter/paginate** |

---

## 📦 Modular Architecture

Scyph is organized as a unified workspace of decoupled crates. You can import the full umbrella crate [`scyph`](https://crates.io/crates/scyph) with feature flags, or import only the individual crates your service needs:

| Crate | Crates.io | Feature Flag | Primary Capabilities |
| :--- | :--- | :--- | :--- |
| **[`scyph-core`](crates/scyph-core)** | [![crates.io](https://img.shields.io/crates/v/scyph-core.svg)](https://crates.io/crates/scyph-core) | *(Always included)* | RFC 7807 `AppError`, `ApiResponse<T>`, `PagedResponse<T>`, `Claims` trait |
| **[`scyph-auth`](crates/scyph-auth)** | [![crates.io](https://img.shields.io/crates/v/scyph-auth.svg)](https://crates.io/crates/scyph-auth) | `auth` | JWT issuance/validation, Argon2id hashing, Moka cache, `AuthUser` extractor |
| **[`scyph-abac`](crates/scyph-abac)** | [![crates.io](https://img.shields.io/crates/v/scyph-abac.svg)](https://crates.io/crates/scyph-abac) | `abac`, `cedar` | Attribute-Based Access Control, SQL query `FilterBuilder`, Cedar policies |
| **[`scyph-db`](crates/scyph-db)** | [![crates.io](https://img.shields.io/crates/v/scyph-db.svg)](https://crates.io/crates/scyph-db) | `db`, `query` | PostgreSQL pool builder, transactions, migrations, dynamic search & filtering |
| **[`scyph-extractors`](crates/scyph-extractors)** | [![crates.io](https://img.shields.io/crates/v/scyph-extractors.svg)](https://crates.io/crates/scyph-extractors) | `extractors` | `ValidatedJson<T>`, `SanitizedJson<T>`, `ValidatedQuery<T>`, `ValidatedPath<T>` |
| **[`scyph-health`](crates/scyph-health)** | [![crates.io](https://img.shields.io/crates/v/scyph-health.svg)](https://crates.io/crates/scyph-health) | `health` | Kubernetes `/health/live` & `/health/ready` endpoint registries |
| **[`scyph-notify`](crates/scyph-notify)** | [![crates.io](https://img.shields.io/crates/v/scyph-notify.svg)](https://crates.io/crates/scyph-notify) | `notify`, `email`, `fcm` | SMTP delivery via Lettre & Tera templates, Firebase Cloud Messaging (FCM) |
| **[`scyph-ratelimit`](crates/scyph-ratelimit)** | [![crates.io](https://img.shields.io/crates/v/scyph-ratelimit.svg)](https://crates.io/crates/scyph-ratelimit) | `ratelimit` | IP & key rate-limiting with reverse proxy header detection (`X-Forwarded-For`) |
| **[`scyph-realtime`](crates/scyph-realtime)** | [![crates.io](https://img.shields.io/crates/v/scyph-realtime.svg)](https://crates.io/crates/scyph-realtime) | `realtime` | Redis Pub/Sub WebSocket broadcaster with per-session bounded message queues |
| **[`scyph-storage`](crates/scyph-storage)** | [![crates.io](https://img.shields.io/crates/v/scyph-storage.svg)](https://crates.io/crates/scyph-storage) | `storage`, `s3`, `image` | AWS S3 & MinIO streaming multipart upload extractors & thumbnail generation |
| **[`scyph-telemetry`](crates/scyph-telemetry)** | [![crates.io](https://img.shields.io/crates/v/scyph-telemetry.svg)](https://crates.io/crates/scyph-telemetry) | `telemetry` | Non-blocking tracing, UUIDv7 request ID tracking, auto-meta JSON injection |
| **[`scyph-utils`](crates/scyph-utils)** | [![crates.io](https://img.shields.io/crates/v/scyph-utils.svg)](https://crates.io/crates/scyph-utils) | `utils` | HMAC webhook verification, background worker tasks, Redis idempotency |

---

## ⚡ Quickstart Example

Add `scyph` to your `Cargo.toml`:

```toml
[dependencies]
scyph = { version = "0.1", features = ["full"] }
tokio = { version = "1", features = ["full"] }
axum = "0.8"
```

Create your Axum service in `src/main.rs`:

```rust
use axum::{routing::get, Router};
use scyph::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize non-blocking structured telemetry logging
    let _guard = init_tracing().expect("Tracing subscriber initialized");

    // 2. Build your standard Axum router
    let app = Router::new()
        .route("/api/hello", get(hello_handler));

    // 3. Wrap router with Scyph telemetry:
    //    - Tracing spans & request logging
    //    - Automatic time-ordered UUIDv7 request ID propagation
    //    - Safe JSON response metadata injection
    //    - Dynamic HTTP response compression (gzip, zstd, brotli)
    let app = with_telemetry(app);

    // 4. Run native Tokio TCP listener
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

// Handlers use Scyph extractors & RFC 7807 response envelopes
async fn hello_handler(RequestId(req_id): RequestId) -> ApiResponse<String> {
    ApiResponse::ok(format!("Hello! Your Request ID is {req_id}"))
}
```

---

## 🎛️ Feature Flags

| Feature | Default | Description |
| :--- | :---: | :--- |
| `default` | Yes | Enables `auth`, `db`, `health`, `telemetry`, `extractors`, `utils` |
| `full` | No | Enables all crates and sub-features across the entire framework |
| `auth` | Yes | JWT token generation, Argon2id hashing, Moka cache, role-based guards |
| `db` | Yes | PostgreSQL connection pooling and transaction lifecycle helpers |
| `query` | No | Dynamic `QueryBuilder` filtering, sorting, searching, and pagination |
| `health` | Yes | Kubernetes liveness/readiness endpoint registries |
| `telemetry` | Yes | Non-blocking tracing, UUIDv7 request IDs, and auto-meta response injection |
| `extractors` | Yes | Sanitized and validated JSON body, query, and path extractors |
| `utils` | Yes | HMAC webhooks, background worker tasks, Redis idempotency keys |
| `abac` | No | Attribute-Based Access Control policies and SQL filter builder |
| `cedar` | No | Amazon Cedar policy engine evaluation in `scyph-abac` |
| `notify` | No | Notification abstractions |
| `email` | No | SMTP transactional emails via Lettre & Tera templates |
| `fcm` | No | Firebase Cloud Messaging push notifications |
| `realtime` | No | Redis Pub/Sub WebSocket broadcaster |
| `storage` | No | Object storage traits and multipart upload extractors |
| `s3` | No | AWS S3 and MinIO backend in `scyph-storage` |
| `image` | No | Automatic image thumbnail generation and resizing |
| `ratelimit` | No | IP-based rate limiting middleware |

---

## 🛡️ License

Dual-licensed under either:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
