#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

mod config;
mod connection;
mod handlers;
mod request;
mod server;
mod time;

use anyhow::Context;

use tracing::info;

fn log_vergen_info() {
    info!("Vergen Info:");
    info!("Git Describe: {}", env!("VERGEN_GIT_DESCRIBE"));
    info!("Git SHA: {}", env!("VERGEN_GIT_SHA"));
    info!(
        "Cargo Target Triple: {}",
        env!("VERGEN_CARGO_TARGET_TRIPLE")
    );
    info!("Build Timestamp: {}", env!("VERGEN_BUILD_TIMESTAMP"));
    info!("Rustc Semver: {}", env!("VERGEN_RUSTC_SEMVER"));
    info!("Sysinfo Name: {}", env!("VERGEN_SYSINFO_NAME"));
    info!(
        "Sysinfo CPU Core Count: {}",
        env!("VERGEN_SYSINFO_CPU_CORE_COUNT"),
    );
}

fn app_name() -> String {
    std::env::args().nth(0).unwrap_or("[UNKNOWN]".to_owned())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    log_vergen_info();

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
