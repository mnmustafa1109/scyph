use scyph_realtime::{
    ConnectionRegistry, RealtimeBroadcaster, RealtimeConfig, RealtimeEvent, RealtimePayload,
    decode_event,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn test_connection_registry_lifecycle() {
    let registry = ConnectionRegistry::default();
    let user_id = Uuid::new_v4();
    let conn_id = Uuid::new_v4();
    let (tx, mut rx) = unbounded_channel();

    assert_eq!(registry.active_connections_count().await, 0);

    // 1. Register connection
    registry.register(user_id, conn_id, tx).await;
    assert_eq!(registry.active_connections_count().await, 1);
    assert_eq!(registry.user_connection_count(user_id).await, 1);

    // 2. Broadcast local message
    registry
        .broadcast_local(user_id, r#"{"hello":"world"}"#)
        .await;

    let received = rx.recv().await;
    assert_eq!(received.as_deref(), Some(r#"{"hello":"world"}"#));

    // 3. Deregister connection (auto purges empty user entries)
    registry.deregister(user_id, conn_id).await;
    assert_eq!(registry.active_connections_count().await, 0);
    assert_eq!(registry.user_connection_count(user_id).await, 0);
}

#[tokio::test]
async fn test_realtime_session_raii_guard() {
    let config = RealtimeConfig::default();
    let broadcaster = RealtimeBroadcaster::new(config).expect("broadcaster creation");
    let user_id = Uuid::new_v4();

    assert_eq!(broadcaster.registry().active_connections_count().await, 0);

    {
        let mut session = broadcaster.connect_session(user_id).await;
        assert_eq!(session.user_id(), user_id);
        assert_eq!(broadcaster.registry().active_connections_count().await, 1);

        broadcaster
            .registry()
            .broadcast_local(user_id, r#"{"msg":"test_session"}"#)
            .await;

        let msg = session.recv().await;
        assert_eq!(msg.as_deref(), Some(r#"{"msg":"test_session"}"#));
    } // session dropped here

    // Give asynchronous drop task a moment to execute
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(broadcaster.registry().active_connections_count().await, 0);
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct UserNotificationPayload {
    pub message: String,
}

impl RealtimePayload for UserNotificationPayload {
    const EVENT_NAME: &'static str = "notification.user";
}

#[test]
fn test_realtime_payload_trait() {
    let payload = UserNotificationPayload {
        message: "Hello World".to_string(),
    };
    assert_eq!(UserNotificationPayload::EVENT_NAME, "notification.user");

    let user_id = Uuid::new_v4();
    let event = RealtimeEvent::new(UserNotificationPayload::EVENT_NAME, user_id, &payload);
    assert_eq!(event.event_name, "notification.user");
    assert_eq!(event.user_id, Some(user_id));
    assert!(event.is_global);
}

#[tokio::test]
async fn test_global_broadcast_registry() {
    let registry = ConnectionRegistry::default();
    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();

    let (tx1, mut rx1) = unbounded_channel();
    let (tx2, mut rx2) = unbounded_channel();

    registry.register(user1, Uuid::new_v4(), tx1).await;
    registry.register(user2, Uuid::new_v4(), tx2).await;

    registry.broadcast_global(r#"{"global":"alert"}"#).await;

    assert_eq!(rx1.recv().await.as_deref(), Some(r#"{"global":"alert"}"#));
    assert_eq!(rx2.recv().await.as_deref(), Some(r#"{"global":"alert"}"#));
}

#[test]
fn test_realtime_event_serialization_and_decoding() {
    let user_id = Uuid::new_v4();
    let payload = json!({ "msg": "Hello Realtime" });
    let event = RealtimeEvent::new("chat.message", user_id, payload.clone());

    let raw = serde_json::to_string(&event).expect("serialization failed");
    let decoded: RealtimeEvent<serde_json::Value> =
        decode_event(&raw).expect("decoding event failed");

    assert_eq!(decoded.event_name, "chat.message");
    assert_eq!(decoded.user_id, Some(user_id));
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_realtime_config_from_env_defaults() {
    unsafe {
        std::env::remove_var("REALTIME_REDIS_URL");
        std::env::remove_var("REDIS_URL");
        std::env::remove_var("REALTIME_CHANNEL_PREFIX");
        std::env::remove_var("REALTIME_RECONNECT_INTERVAL_SECS");
    }

    let config = RealtimeConfig::from_env().expect("config from env should succeed");
    assert_eq!(config.redis_url, "redis://127.0.0.1:6379");
    assert_eq!(config.channel_prefix, "scyph:realtime");
    assert_eq!(config.reconnect_interval_secs, 1);
}

#[tokio::test]
async fn test_subscriber_cancellation_token() {
    let config = RealtimeConfig::default();
    let broadcaster = RealtimeBroadcaster::new(config).expect("broadcaster creation");
    let cancel_token = CancellationToken::new();

    let handle = broadcaster.start_subscriber(cancel_token.clone());

    // Cancel token and verify handle completes
    cancel_token.cancel();
    let _ = handle.await;
}

#[cfg(feature = "health")]
#[tokio::test]
async fn test_realtime_health_check_integration() {
    use scyph_health::HealthRegistry;
    use scyph_realtime::RealtimeHealthExt;

    let config = RealtimeConfig::default();
    let broadcaster = RealtimeBroadcaster::new(config).expect("broadcaster creation");
    let registry = HealthRegistry::new();

    broadcaster
        .check_health_named(&registry, "test_realtime", false)
        .await;

    let snapshot = registry.snapshot().await;
    assert!(snapshot.contains_key("test_realtime"));
    let service_status = &snapshot["test_realtime"];
    assert_eq!(service_status.required, false);
}
