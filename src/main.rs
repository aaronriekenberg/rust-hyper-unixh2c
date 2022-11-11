#![warn(rust_2018_idioms)]

mod config;
mod handlers;
mod request;
mod server;

use anyhow::Context;

fn app_name() -> String {
    std::env::args().nth(0).unwrap_or("[UNKNOWN]".to_owned())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config_file = std::env::args().nth(1).with_context(|| {
        format!(
            "config file required as command line argument: {} <config file>",
            app_name(),
        )
    })?;

    crate::config::read_configuration(config_file)
        .await
        .context("read_configuration error")?;

    let handlers = crate::handlers::create_handlers()?;

    let server = crate::server::Server::new(handlers);

    server.run().await
}
