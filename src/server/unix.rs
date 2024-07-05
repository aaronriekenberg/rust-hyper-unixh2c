use anyhow::Context;

use hyper_util::rt::TokioIo;

use tracing::{debug, info};

use tokio::net::UnixListener;

use std::sync::Arc;

use crate::{
    config::ServerSocketType, server::handler::ConnectionHandler,
    service::connection::ConnectionTrackerService,
};

pub struct UnixServer {
    connection_handler: Arc<ConnectionHandler>,
    connection_tracker: &'static ConnectionTrackerService,
    listener_configuration: &'static crate::config::ServerListenerConfiguration,
}

impl UnixServer {
    pub async fn new(
        connection_handler: Arc<ConnectionHandler>,
        listener_configuration: &'static crate::config::ServerListenerConfiguration,
    ) -> Self {
        Self {
            connection_handler,
            connection_tracker: ConnectionTrackerService::instance().await,
            listener_configuration,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let path = &self.listener_configuration.bind_address;

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
                .add_connection(ServerSocketType::Unix)
                .await
            {
                self.connection_handler
                    .start_connection_handler(TokioIo::new(unix_stream), connection);
            }
        }
    }
}
