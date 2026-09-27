# 🗺️ Scyph Project Roadmap

> **Vision**: Scyph is a modular, high-performance toolkit of Rust crates designed for building production-ready, scalable web backends with [Axum](https://github.com/tokio-rs/axum).

This document outlines the development roadmap, planned features, and architectural goals for the Scyph workspace.

---

## 📊 Status Legend

- 🟢 **Completed**: Delivered and verified in `v0.1.0`.
- 🟡 **In Development / High Priority**: Targeted for upcoming `v0.2.0` releases.
- 🔵 **Planned**: Scheduled for future minor releases (`v0.3.0+`).
- 🟣 **Under Evaluation**: Conceptual features open for community feedback.

---

## 🚀 Release Milestones

```
  v0.1.0 (Current)            v0.2.0 (Next Target)                v0.3.0+ (Future Horizon)
┌───────────────────────┐   ┌────────────────────────────────┐   ┌───────────────────────────┐
│ • Core Abstractions   │   │ • Multi-Instance Auth          │   │ • Stripe & IAP Billing    │
│ • Single-node Moka    │──►│   Blacklisting (Moka + Redis)  │──►│ • OpenTelemetry OTLP      │
│ • S3 Dual-Client      │   │ • Hybrid L1/L2 Cache Engine    │   │ • Cedar Policy Caching    │
│ • Axum Extractors     │   │ • Redis Pub/Sub Broadcast      │   │ • Distributed Rate Limit  │
└───────────────────────┘   └────────────────────────────────┘   └───────────────────────────┘
```

---

## 🎯 Feature Breakdown by Crate

### 🔐 `scyph-auth` (Authentication & Security)

- 🟢 **Argon2id Password Hashing**: Async, thread-pool managed password hashing with cost parameter configuration.
- 🟢 **JWT Signing & Verification**: Fast token issue/verify using `jsonwebtoken` and `aws_lc_rs`.
- 🟢 **Single-Node In-Memory Cache**: Local Moka cache (`AuthCacheService`) for user profile caching and token revocation.
- 🟡 **Multi-Instance Auth Blacklisting**:
  - Extend `AuthCacheService` to support horizontal scaling across multiple backend application instances.
  - Implement real-time token revocation propagation using a **Redis Pub/Sub channel** to notify peer instances when `blacklist_token()` is invoked.
  - Provide a fallback invalidation strategy when Redis is temporarily unavailable.
- 🟡 **Hybrid L1/L2 Cache Architecture**:
  - **L1 (Local)**: High-speed, lock-free in-memory caching powered by [`moka`](https://crates.io/crates/moka).
  - **L2 (Remote)**: Shared Redis backend for cross-node cache synchronization, persistent session revocation, and distributed profile retrieval.
- 🔵 **OIDC / OAuth2 Provider Helpers**: Built-in extractors and flow handlers for Google, GitHub, and custom OIDC providers.

---

### 🛡️ `scyph-abac` (Attribute-Based Access Control)

- 🟢 **Cedar Policy Integration**: Built-in evaluation of Amazon Cedar policy definitions.
- 🟢 **Dynamic Filter Builder**: Strongly typed query filter construct for ABAC policy enforcement.
- 🔵 **Policy Cache**: In-memory compilation and caching of Cedar policy slices for sub-millisecond evaluation.

---

### 🗄️ `scyph-storage` (Object Storage & Files)

- 🟢 **Dual-Client S3 Architecture**: Separate client configurations for internal cluster communication (e.g. MinIO) and public presigned CDN URLs.
- 🟢 **Multipart File Extractors**: Type-safe Axum extractors (`FileExtractor`, `MultiFileExtractor`) with size/MIME validation.
- 🟡 **Resumable Uploads**: Support for S3 multipart upload session initiation, chunk streaming, and completion.
- 🔵 **Image Processing Middleware**: Optional feature flag for automated thumbnail generation and format conversion upon upload.

---

### 🚦 `scyph-ratelimit` (Rate Limiting)

- 🟢 **Tower/Governor Integration**: Per-IP, strict, and relaxed rate-limiting layers.
- 🟡 **Distributed Rate Limiting**: Redis-backed sliding window rate limiter for multi-instance load balancer setups.

---

### 📡 `scyph-realtime` & `scyph-notify` (Realtime & Notifications)

- 🟢 **Redis Pub/Sub WebSocket Broadcaster**: Multi-replica event broadcasting (`RealtimeBroadcaster`) using Redis Pub/Sub for cross-instance WebSocket message fanout.
- 🟢 **WebSocket Connection Registry & Sessions**: Connection tracking, RAII session guards, and user-scoped messaging.
- 🟢 **Email & Push Dispatcher**: Lettre SMTP, FCM push, and composite notification routing.
- 🔵 **NATS / RabbitMQ Adapters**: Optional message broker backends alongside Redis.

---

### 📊 `scyph-telemetry` & `scyph-health` (Observability)

- 🟢 **Structured Tracing**: JSON & ANSI tracing subscriber with request ID propagation.
- 🟢 **Kubernetes Health Probes**: `/livez` and `/readyz` route handlers with component health registries.
- 🔵 **OpenTelemetry OTLP Exporter**: Direct metrics and trace export to Jaeger, Prometheus, or Datadog.

---

### ⚙️ `scyph-jobs` (Background Job Processing & Workflows)

- 🟡 **Dedicated Draft Branch (`scyph-job`)**: Maintained on the `scyph-job` branch pending stable `apalis` v1.0 release before workspace merging.
- 🟢 **PostgreSQL Job Queue**: `apalis`-backed scheduled and delayed background task queue runner.
- 🟢 **Worker Lifecycle & Cancellation**: Worker execution runner with custom tick behaviors and graceful task cancellation.
- 🔵 **Distributed Job Metrics & Dashboard**: Prometheus metrics export and queue monitoring endpoints.

---

### 💳 `scyph-pay` (Payments, Subscriptions & In-App Purchases)

- 🔵 **Stripe Integration**:
  - Axum webhook signature verification extractor (`StripeEvent`).
  - Checkout session creation, customer portal integration, and recurring subscription lifecycle handlers.
- 🔵 **In-App Purchases (IAP)**:
  - **Apple App Store**: App Store Server API v2 receipt validation, JWT transaction verification, and Server Notifications v2 webhook listener.
  - **Google Play Billing**: Google Play Developer API purchase token verification and Real-time Developer Notifications (RTDN) listener.
  - **Unified Entitlement Engine**: Trait abstraction for cross-platform entitlement checks (iOS, Android, Web).

---

## 🤝 Contributing to the Roadmap

We welcome ideas, feature requests, and community feedback!
- To request a new feature or suggest an architectural improvement, open an issue on GitHub tagged `enhancement`.
- To discuss implementation details for items on this roadmap (such as the **Multi-Instance Auth Blacklisting**), join the discussion threads on GitHub.
