use scyph_db::{run_migrations_from, run_seeds_from};
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_run_migrations_nonexistent_dir() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/test_db")
        .expect("lazy pool creation should succeed");

    let result = run_migrations_from(&pool, "non_existent_migrations_dir_999").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_seeds_disabled() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/test_db")
        .expect("lazy pool creation should succeed");

    unsafe {
        std::env::set_var("RUN_SEEDS", "false");
    }

    let result = run_seeds_from(&pool, "non_existent_seeds_dir_999").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_seeds_nonexistent_dir() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/test_db")
        .expect("lazy pool creation should succeed");

    unsafe {
        std::env::set_var("RUN_SEEDS", "true");
    }

    let result = run_seeds_from(&pool, "non_existent_seeds_dir_999").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_migrations_malformed_file() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost:5432/test_db")
        .expect("lazy pool creation should succeed");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("scyph_db_test_bad_mig_{now}"));
    let _ = std::fs::create_dir_all(&temp_dir);
    let bad_file = temp_dir.join("invalid_filename_format.sql");
    let _ = std::fs::write(&bad_file, "SELECT 1;");

    let result = run_migrations_from(&pool, &temp_dir).await;
    // Malformed migration file names must fail with an error and not be silently swallowed
    assert!(result.is_err());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
