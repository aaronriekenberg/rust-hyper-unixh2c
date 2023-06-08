use anyhow::Context;
use tracing::{info, warn};

use tokio::net::TcpListener;

use std::sync::Arc;

use crate::{config::ServerSocketType, connection::ConnectionTracker};

use super::ConnectionHandler;

pub struct TCPServer {
    connection_handler: Arc<ConnectionHandler>,
    connection_tracker: &'static ConnectionTracker,
    server_configuration: &'static crate::config::ServerConfiguration,
}

impl TCPServer {
    pub(super) async fn new(
        connection_handler: Arc<ConnectionHandler>,
        server_configuration: &'static crate::config::ServerConfiguration,
    ) -> Self {
        Self {
            connection_handler,
            connection_tracker: ConnectionTracker::instance().await,
            server_configuration,
        }
    }

    pub(super) async fn run(self) -> anyhow::Result<()> {
        let address = self.server_configuration.bind_address();

        let tcp_listener = TcpListener::bind(address)
            .await
            .with_context(|| format!("TCP server bind error address = {:?}", address))?;

        let local_addr = tcp_listener
            .local_addr()
            .with_context(|| format!("TCP server local_addr error address = {:?}", address))?;

        info!("listening on tcp {:?}", local_addr);

        loop {
            let (tcp_stream, _remote_addr) = tcp_listener.accept().await?;

            if let Err(e) = tcp_stream.set_nodelay(true) {
                warn!("error setting tcp no delay {:?}", e);
                continue;
            };

            let connection = self
                .connection_tracker
                .add_connection(
                    *self.server_configuration.server_protocol(),
                    ServerSocketType::Tcp,
                )
                .await;

            tokio::task::spawn(
                Arc::clone(&self.connection_handler).handle_connection(tcp_stream, connection),
            );
        }
    }
}
