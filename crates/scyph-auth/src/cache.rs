//! In-memory authentication and profile caching powered by [`moka`].
//!
//! Provides [`AuthCacheService`] to cache user profiles in memory (reducing database hits)
//! and maintain a JWT revocation blacklist using unique token identifiers (`jti`).
//!
//! # Caching Strategy
//!
//! `AuthCacheService` uses two separate [`moka::future::Cache`] instances:
//!
//! - **Profile cache** (`Uuid → P`): Stores application-defined profile structs (e.g. `UserProfile`),
//!   keyed by user UUID. Reduces repeated database queries for frequently accessed user data.
//!   Profiles expire after `profile_ttl_secs` seconds of inactivity.
//!
//! - **Blacklist cache** (`String → ()`): Stores revoked JWT `jti` values. When a token is
//!   revoked (logout, password change, admin action), its `jti` is inserted here. The extractor
//!   checks this cache on every request. Entries expire after `blacklist_ttl_secs` seconds,
//!   which should be set to the maximum possible JWT lifetime to ensure revocation persists
//!   until natural token expiry.
//!
//! # Full Workflow Example
//!
//! ```rust,ignore
//! use scyph_auth::AuthCacheService;
//! use uuid::Uuid;
//!
//! #[derive(Clone)]
//! struct UserProfile {
//!     id: Uuid,
//!     email: String,
//!     is_active: bool,
//! }
//!
//! # #[tokio::main]
//! # async fn main() {
//! // Build a cache: up to 10k profiles, 5-minute profile TTL, 1-hour blacklist TTL
//! let cache = AuthCacheService::<UserProfile>::new(10_000, 300, 3600);
//!
//! let user_id = Uuid::now_v7();
//! let profile = UserProfile {
//!     id: user_id,
//!     email: "alice@example.com".into(),
//!     is_active: true,
//! };
//!
//! // Cache the user profile after a successful database lookup
//! cache.set_profile(user_id, profile).await;
//!
//! // Fast path: retrieve profile from cache (no DB hit)
//! if let Some(p) = cache.get_profile(&user_id).await {
//!     assert_eq!(p.email, "alice@example.com");
//! }
//!
//! // Simulate a logout: blacklist the token's jti
//! let jti = "token-unique-id-abc123";
//! cache.blacklist_token(jti).await;
//! assert!(cache.is_token_revoked(jti).await);
//!
//! // When user's role changes, invalidate their profile cache
//! cache.invalidate_profile(&user_id).await;
//! assert!(cache.get_profile(&user_id).await.is_none());
//! # }
//! ```
//!
//! # Concurrency
//!
//! `AuthCacheService<P>` implements `Clone` — cloning it shares the underlying cache between
//! Axum handler tasks. It is safe to hold in an `Arc<AppState>` or as a plain field in a
//! `#[derive(Clone)]` state struct.

use moka::future::Cache;
use std::time::Duration;
use uuid::Uuid;

/// High-performance in-memory authentication cache service.
///
/// `AuthCacheService` caches security profiles of generic type `P` indexed by User [`Uuid`],
/// and maintains an expiration-based revocation list for revoked JWT tokens (identified by `jti`).
///
/// # Type Parameters
/// - `P`: Your application's security profile struct (e.g. `UserProfile`).
#[derive(Clone)]
pub struct AuthCacheService<P: Clone + Send + Sync + 'static> {
    profile: Cache<Uuid, P>,
    blacklist: Cache<String, ()>,
}

impl<P: Clone + Send + Sync + 'static> AuthCacheService<P> {
    /// Creates a new `AuthCacheService` with specified capacity, profile TTL, and token blacklist TTL.
    ///
    /// # Arguments
    ///
    /// * `max_capacity` - Maximum number of profile records and revoked tokens to store in memory.
    /// * `profile_ttl_secs` - Time-to-live in seconds for cached user profiles.
    /// * `blacklist_ttl_secs` - Time-to-live in seconds for revoked JWT tokens (should match or exceed maximum JWT expiration).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_auth::AuthCacheService;
    ///
    /// #[derive(Clone)]
    /// struct UserProfile { id: uuid::Uuid, is_active: bool }
    ///
    /// let cache = AuthCacheService::<UserProfile>::new(10_000, 300, 3600);
    /// ```
    pub fn new(max_capacity: u64, profile_ttl_secs: u64, blacklist_ttl_secs: u64) -> Self {
        Self {
            profile: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(profile_ttl_secs))
                .build(),
            blacklist: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(blacklist_ttl_secs))
                .build(),
        }
    }

    /// Stores a user security profile in the in-memory cache.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the user.
    /// * `p` - Profile data to store.
    pub async fn set_profile(&self, id: Uuid, p: P) {
        self.profile.insert(id, p).await;
    }

    /// Retrieves a cached user profile by user UUID.
    ///
    /// Returns `Some(P)` if the profile exists and has not expired, otherwise `None`.
    ///
    /// # Arguments
    ///
    /// * `id` - Reference to the user UUID to look up.
    pub async fn get_profile(&self, id: &Uuid) -> Option<P> {
        self.profile.get(id).await
    }

    /// Removes a user security profile from the cache.
    ///
    /// Useful when user permissions or profiles change in the database.
    ///
    /// # Arguments
    ///
    /// * `id` - Reference to the user UUID to invalidate.
    pub async fn invalidate_profile(&self, id: &Uuid) {
        self.profile.invalidate(id).await;
    }

    /// Adds a JWT identifier (`jti`) to the revocation blacklist cache.
    ///
    /// # Arguments
    ///
    /// * `jti` - Unique JWT token ID string to revoke.
    pub async fn blacklist_token(&self, jti: &str) {
        self.blacklist.insert(jti.to_string(), ()).await;
    }

    /// Checks if a JWT identifier (`jti`) has been revoked.
    ///
    /// Returns `true` if the `jti` is found in the blacklist cache, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `jti` - Unique JWT token ID string to query.
    pub async fn is_token_revoked(&self, jti: &str) -> bool {
        self.blacklist.get(jti).await.is_some()
    }
}
