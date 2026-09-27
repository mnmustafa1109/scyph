// crates/scyph-utils/src/idempotency.rs
use redis::AsyncCommands;
use scyph_core::error::AppError;
use std::time::Duration;

pub enum IdempotencyCheck {
    New,
    Seen(String),
}

pub struct IdempotencyStore {
    redis: redis::Client,
    ttl: Duration,
}

impl IdempotencyStore {
    pub fn new(redis_url: &str, ttl: Duration) -> Result<Self, AppError> {
        let redis = redis::Client::open(redis_url)
            .map_err(|e| AppError::internal_from(e, "connect Redis"))?;
        Ok(Self { redis, ttl })
    }

    pub async fn begin(&self, key: &str) -> Result<IdempotencyCheck, AppError> {
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::internal_from(e, "Redis connection"))?;
        let redis_key = format!("idem:{key}");
        let is_new: bool = conn
            .set_nx(&redis_key, "")
            .await
            .map_err(|e| AppError::internal_from(e, "Redis SETNX"))?;
        if is_new {
            let _: () = conn
                .expire(&redis_key, self.ttl.as_secs() as i64)
                .await
                .map_err(|e| AppError::internal_from(e, "Redis EXPIRE"))?;
            Ok(IdempotencyCheck::New)
        } else {
            let cached: String = conn
                .get(&redis_key)
                .await
                .map_err(|e| AppError::internal_from(e, "Redis GET"))?;
            Ok(IdempotencyCheck::Seen(cached))
        }
    }

    pub async fn complete(&self, key: &str, response_body: &str) -> Result<(), AppError> {
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::internal_from(e, "Redis connection"))?;
        let redis_key = format!("idem:{key}");
        let _: () = conn
            .set_ex(&redis_key, response_body, self.ttl.as_secs())
            .await
            .map_err(|e| AppError::internal_from(e, "Redis SETEX"))?;
        Ok(())
    }
}
