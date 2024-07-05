mod config;
mod handlers;
mod request;
mod response;
mod server;
mod service;
mod tracing_config;
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

    crate::service::connection::ConnectionTrackerService::instance().await;

    crate::service::static_file::create_rules_service_instance()?;

    let handlers = handlers::create_handlers().await?;

    let server = crate::server::Server::new(handlers).await;

    server.run().await
}

#[tokio::main]
async fn main() {
    tracing_config::initialize_tracing_subscriber();

    if let Err(err) = try_main().await {
        error!("fatal error in main:\n{:#}", err);
        std::process::exit(1);
    }
}
