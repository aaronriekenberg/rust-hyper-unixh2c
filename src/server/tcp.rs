use anyhow::Context;

use tracing::{info, warn};

use tokio::net::TcpListener;

use std::sync::Arc;

use crate::{
    config::ServerSocketType, connection::ConnectionTracker, server::connection::ConnectionHandler,
};

pub struct TCPServer {
    connection_handler: Arc<ConnectionHandler>,
    connection_tracker: &'static ConnectionTracker,
    listener_configuration: &'static crate::config::ServerListenerConfiguration,
}

impl TCPServer {
    pub async fn new(
        connection_handler: Arc<ConnectionHandler>,
        listener_configuration: &'static crate::config::ServerListenerConfiguration,
    ) -> Self {
        Self {
            connection_handler,
            connection_tracker: ConnectionTracker::instance().await,
            listener_configuration,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let address = self.listener_configuration.bind_address();

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

            if let Some(connection) = self
                .connection_tracker
                .add_connection(
                    self.listener_configuration.server_protocol(),
                    ServerSocketType::Tcp,
                )
                .await
            {
                tokio::task::spawn(
                    Arc::clone(&self.connection_handler).handle_connection(tcp_stream, connection),
                );
            }
        }
    }
}
