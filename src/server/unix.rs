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
    listener_configuration: &'static crate::config::ServerListenerConfiguration,
}

impl UnixServer {
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
        let path = self.listener_configuration.bind_address();

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

            if let Some(connection) = self
                .connection_tracker
                .add_connection(
                    self.listener_configuration.server_protocol(),
                    ServerSocketType::Unix,
                )
                .await
            {
                tokio::task::spawn(
                    Arc::clone(&self.connection_handler).handle_connection(unix_stream, connection),
                );
            }
        }
    }
}
