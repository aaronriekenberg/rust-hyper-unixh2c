#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

mod config;
mod connection;
mod handlers;
mod request;
mod response;
mod server;
mod time;
mod version;

use anyhow::Context;

use tracing::{error, info, instrument};

async fn log_version_info() {
    info!("Version Info:");
    for (key, value) in version::get_verison_info().await {
        info!("{}: {}", key, value);
    }
}

fn app_name() -> String {
    std::env::args().next().unwrap_or("[UNKNOWN]".to_owned())
}

#[instrument]
async fn try_main() -> anyhow::Result<()> {
    log_version_info().await;

    let config_file = std::env::args().nth(1).with_context(|| {
        format!(
            "config file required as command line argument: {} <config file>",
            app_name(),
        )
    })?;

    crate::config::read_configuration(config_file)
        .await
        .context("read_configuration error")?;

    let handlers = handlers::create_handlers().await?;

    let server = crate::server::Server::new(handlers).await;

    server.run().await
}

fn initialize_tracing_subscriber() {
    use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};

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

#[tokio::main]
async fn main() {
    initialize_tracing_subscriber();

    if let Err(err) = try_main().await {
        error!("fatal error in main:\n{:#}", err);
        std::process::exit(1);
    }
}
