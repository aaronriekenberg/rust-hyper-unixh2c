use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};

pub fn initialize_tracing_subscriber() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let log_format_value = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "dev".to_string());

    if log_format_value.eq_ignore_ascii_case("prod") {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_ansi(false).without_time())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer())
            .init();
    };
}
