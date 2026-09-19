// crates/skidblad-auth/src/cache.rs
use moka::future::Cache;
use std::time::Duration;
use uuid::Uuid;

/// In-memory auth cache backed by moka.
/// `P` is your project's user security profile (e.g. `{ id, role, is_active }`).
#[derive(Clone)]
pub struct AuthCacheService<P: Clone + Send + Sync + 'static> {
    profile: Cache<Uuid, P>,
    blacklist: Cache<String, ()>,
}

impl<P: Clone + Send + Sync + 'static> AuthCacheService<P> {
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

    pub async fn set_profile(&self, id: Uuid, p: P) {
        self.profile.insert(id, p).await;
    }
    pub async fn get_profile(&self, id: &Uuid) -> Option<P> {
        self.profile.get(id).await
    }
    pub async fn invalidate_profile(&self, id: &Uuid) {
        self.profile.invalidate(id).await;
    }

    pub async fn blacklist_token(&self, jti: &str) {
        self.blacklist.insert(jti.to_string(), ()).await;
    }
    pub async fn is_token_revoked(&self, jti: &str) -> bool {
        self.blacklist.get(jti).await.is_some()
    }
}
