use futures_util::StreamExt;
use redis::AsyncCommands;
use redis::Client;
use scyph_core::AppError;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{sync::RwLock, sync::mpsc::UnboundedSender, time::sleep};
use tracing::{error, info};
use uuid::Uuid;

const REDIS_CHANNEL: &str = "scyph-realtime";

pub type ConnectionTx = UnboundedSender<String>;

pub type UserConnections = HashMap<Uuid, ConnectionTx>;

pub type ConnectionRegistry = Arc<RwLock<HashMap<Uuid, UserConnections>>>;

#[derive(Debug, Clone)]
pub struct RealtimeBroadcaster {
    client: Client,
    local: ConnectionRegistry,
}

impl RealtimeBroadcaster {
    pub fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = Client::open(redis_url)
            .map_err(|e| AppError::internal_from(e, "Failed to connect to Redis"))?;
        Ok(Self {
            client,
            local: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn register(&self, user_id: Uuid, conn_id: Uuid, tx: UnboundedSender<String>) {
        self.local
            .write()
            .await
            .entry(user_id)
            .or_default()
            .insert(conn_id, tx);
    }

    pub async fn deregister(&self, user_id: Uuid, conn_id: Uuid) {
        if let Some(conns) = self.local.write().await.get_mut(&user_id) {
            conns.remove(&conn_id);
        }
    }
    pub async fn publish<T: Serialize>(&self, user_id: Uuid, event: &T) -> Result<(), AppError> {
        let payload = serde_json::to_string(&RealtimeEnvelope {
            user_id,
            body: event,
        })
        .map_err(|e| AppError::internal_from(e, "serialize event"))?;
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::internal_from(e, "Redis connection"))?;
        conn.publish::<_, _, ()>(REDIS_CHANNEL, payload)
            .await
            .map_err(|e| AppError::internal_from(e, "Redis PUBLISH"))?;
        Ok(())
    }

    async fn deliver_local(&self, user_id: Uuid, raw: &str) {
        if let Some(conns) = self.local.read().await.get(&user_id) {
            for tx in conns.values() {
                let _ = tx.send(raw.to_string());
            }
        }
    }

    pub fn spawn_subscriber(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.run_subscriber_once().await {
                    error!(error = %e, "Realtime subscriber errored, reconnecting in 1s");
                    sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        })
    }

    async fn run_subscriber_once(&self) -> Result<(), AppError> {
        let mut pubsub = self
            .client
            .get_async_pubsub()
            .await
            .map_err(|e| AppError::internal_from(e, "Redis connection"))?;
        pubsub
            .subscribe(REDIS_CHANNEL)
            .await
            .map_err(|e| AppError::internal_from(e, "Redis SUBSCRIBE"))?;
        info!("Realtime subscriber connected");

        let mut stream = pubsub.on_message();
        while let Some(msg) = stream.next().await {
            let raw: String = msg.get_payload().unwrap_or_default();
            if let Ok(env) = serde_json::from_str::<RealtimeEnvelopeRaw>(&raw) {
                self.deliver_local(env.user_id, &raw).await;
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct RealtimeEnvelope<'a, T: Serialize> {
    user_id: Uuid,
    body: &'a T,
}

#[derive(serde::Deserialize)]
struct RealtimeEnvelopeRaw {
    user_id: Uuid,
}

pub fn decode_event<T: DeserializeOwned>(raw: &str) -> Result<T, AppError> {
    #[derive(serde::Deserialize)]
    struct Wrapper<T> {
        body: T,
    }
    let w: Wrapper<T> =
        serde_json::from_str(raw).map_err(|e| AppError::internal_from(e, "decode event"))?;
    Ok(w.body)
}
