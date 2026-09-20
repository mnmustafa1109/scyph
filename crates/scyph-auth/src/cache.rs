//! In-memory authentication and profile caching powered by [`moka`].
//!
//! Provides [`AuthCacheService`] to cache user profiles in memory (reducing database hits)
//! and maintain a JWT revocation blacklist using unique token identifiers (`jti`).

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
    /// Creates a new `AuthCacheService` with specified capacity and profile TTL.
    ///
    /// The token blacklist cache is automatically initialized with a max capacity of `max_capacity`
    /// and a default TTL of 1 hour (3600 seconds).
    ///
    /// # Arguments
    ///
    /// * `max_capacity` - Maximum number of profile records and revoked tokens to store in memory.
    /// * `ttl_secs` - Time-to-live in seconds for cached user profiles.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_auth::AuthCacheService;
    ///
    /// #[derive(Clone)]
    /// struct UserProfile { id: uuid::Uuid, is_active: bool }
    ///
    /// let cache = AuthCacheService::<UserProfile>::new(10_000, 300);
    /// ```
    pub fn new(max_capacity: u64, ttl_secs: u64) -> Self {
        Self {
            profile: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(ttl_secs))
                .build(),
            blacklist: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(3600))
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
