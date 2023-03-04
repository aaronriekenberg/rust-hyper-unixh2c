#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

mod config;
mod connection;
mod context;
mod handlers;
mod request;
mod server;
mod time;

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

    let app_context = crate::context::AppContext::new();

    let handlers = handlers::create_handlers(&app_context)?;

    let server = crate::server::Server::new(app_context.connection_tracker(), handlers);

    server.run().await
}
