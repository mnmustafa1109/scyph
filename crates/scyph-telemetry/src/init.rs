pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = env::var("LOG_FORMAT").unwrap_or_else(|_| "json".into());
}
