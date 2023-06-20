mod connection;
mod tcp;
mod unix;

use anyhow::Context;

use tokio::task::JoinSet;

use std::sync::Arc;

use crate::{config::ServerSocketType, handlers::RequestHandler, request::RequestIDFactory};

use self::{connection::ConnectionHandler, tcp::TCPServer, unix::UnixServer};

pub struct Server {
    join_set: JoinSet<anyhow::Result<()>>,
}

impl Server {
    pub async fn new(handlers: Box<dyn RequestHandler>) -> Self {
        let request_id_factory = RequestIDFactory::new();
        let connection_handler = ConnectionHandler::new(handlers, request_id_factory);

        let mut join_set = JoinSet::new();

        let configuration = crate::config::instance();

        for listener_configuration in configuration.server_configuration().listeners() {
            let connection_handler_clone = Arc::clone(&connection_handler);
            join_set.spawn(async move {
                match listener_configuration.server_socket_type() {
                    ServerSocketType::Tcp => {
                        let server =
                            TCPServer::new(connection_handler_clone, listener_configuration).await;
                        server.run().await?;
                    }
                    ServerSocketType::Unix => {
                        let server =
                            UnixServer::new(connection_handler_clone, listener_configuration).await;
                        server.run().await?;
                    }
                };
                Ok(())
            });
        }

        Self { join_set }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let result = self
            .join_set
            .join_next()
            .await
            .context("join_set.join_next returned None")?;

        let result = result.context("join_next JoinError")?;

        result.context("server.run returned error")?;

        anyhow::bail!("join_set.join_next returned without error");
    }
}
