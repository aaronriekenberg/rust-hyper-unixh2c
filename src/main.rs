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

use tracing::info;

async fn log_version_info() {
    info!("Version Info:");
    for (key, value) in version::get_verison_info().await {
        info!("{}: {}", key, value);
    }
}

fn app_name() -> String {
    std::env::args().next().unwrap_or("[UNKNOWN]".to_owned())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

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
