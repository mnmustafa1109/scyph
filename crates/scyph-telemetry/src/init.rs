use std::env;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = env::var("LOG_FORMAT").unwrap_or_else(|_| "json".into());

    match format.as_str() {
        "pretty" => tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .init(),
        _ => tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init(),
    }
}
