// //! Background queue definitions and worker functions.
//
// use apalis::prelude::*;
// use scyph_core::error::AppError;
//
// #[cfg(feature = "postgres")]
// use apalis_sql::postgres::PostgresStorage;
//
// #[cfg(feature = "redis")]
// use apalis_redis::RedisStorage;
//
// #[cfg(feature = "postgres")]
// /// Configures PostgreSQL storage schema for apalis jobs.
// pub async fn setup_postgres(pool: &sqlx::PgPool) -> Result<(), AppError> {
//     PostgresStorage::<()>::setup(pool)
//         .await
//         .map_err(|e| AppError::internal_from(e, "apalis Postgres setup"))
// }
//
// /// Follow-up reminder payload.
// #[derive(Debug, serde::Serialize, serde::Deserialize)]
// pub struct SendFollowUpReminder {
//     /// Target contact ID.
//     pub contact_id: uuid::Uuid,
//     /// Message content.
//     pub message: String,
// }
//
// /// Handler for processing follow-up reminder jobs.
// pub async fn handle_follow_up_reminder(
//     job: SendFollowUpReminder,
//     _ctx: Data<()>,
// ) -> Result<(), apalis::prelude::BoxDynError> {
//     tracing::info!(contact_id = %job.contact_id, message = %job.message, "Sending follow-up reminder");
//     Ok(())
// }
//
// #[cfg(feature = "redis")]
// /// Spawns a Redis-backed apalis worker loop for follow-up reminders.
// pub fn follow_up_reminder_redis_worker(
//     storage: RedisStorage<SendFollowUpReminder>,
// ) -> impl std::future::Future<Output = ()> {
//     async move {
//         WorkerBuilder::new("follow-up-reminders")
//             .backend(storage)
//             .build_fn(handle_follow_up_reminder)
//             .run()
//             .await;
//     }
// }
