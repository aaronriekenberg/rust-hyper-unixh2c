use anyhow::Context;

use tracing::{debug, info};

use tokio::net::UnixListener;

use std::sync::Arc;

use crate::{
    config::ServerSocketType, connection::ConnectionTracker, server::connection::ConnectionHandler,
};

pub struct UnixServer {
    connection_handler: Arc<ConnectionHandler>,
    connection_tracker: &'static ConnectionTracker,
    server_configuration: &'static crate::config::ServerConfiguration,
}

impl UnixServer {
    pub async fn new(
        connection_handler: Arc<ConnectionHandler>,
        server_configuration: &'static crate::config::ServerConfiguration,
    ) -> Self {
        Self {
            connection_handler,
            connection_tracker: ConnectionTracker::instance().await,
            server_configuration,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let path = self.server_configuration.bind_address();

        // do not fail on remove error, the path may not exist.
        let remove_result = tokio::fs::remove_file(path).await;
        debug!("remove_result = {:?}", remove_result);

        let unix_listener = UnixListener::bind(path)
            .with_context(|| format!("UNIX server bind path = {:?}", path))?;

        let local_addr = unix_listener
            .local_addr()
            .with_context(|| format!("UNIX server local_addr error path = {:?}", path))?;

        info!("listening on unix {:?}", local_addr);

        loop {
            let (unix_stream, _remote_addr) = unix_listener.accept().await?;

            let connection = self
                .connection_tracker
                .add_connection(
                    *self.server_configuration.server_protocol(),
                    ServerSocketType::Unix,
                )
                .await;

            tokio::task::spawn(
                Arc::clone(&self.connection_handler).handle_connection(unix_stream, connection),
            );
        }
    }
}
