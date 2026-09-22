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
