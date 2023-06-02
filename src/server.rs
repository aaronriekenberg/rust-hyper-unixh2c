use anyhow::Context;

use hyper::{
    http::{Request, Response},
    server::conn::http1::Builder as HyperHTTP1Builder,
    server::conn::http2::Builder as HyperHTTP2Builder,
    service::service_fn,
};

use tracing::{debug, info, instrument, warn, Instrument};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, UnixListener},
    task::JoinSet,
};

use std::{convert::Infallible, sync::Arc};

use crate::{
    config::{ServerProtocol, ServerSocketType},
    connection::{ConnectionGuard, ConnectionID, ConnectionTracker},
    handlers::RequestHandler,
    request::{HttpRequest, RequestID, RequestIDFactory},
    response::ResponseBody,
};

struct ConnectionHandler {
    handlers: Box<dyn RequestHandler>,
    request_id_factory: RequestIDFactory,
}

impl ConnectionHandler {
    fn new(handlers: Box<dyn RequestHandler>, request_id_factory: RequestIDFactory) -> Arc<Self> {
        Arc::new(Self {
            handlers,
            request_id_factory,
        })
    }

    #[instrument(skip_all, fields(req_id = request_id.as_usize()))]
    async fn handle_request(
        self: Arc<Self>,
        connection_id: ConnectionID,
        request_id: RequestID,
        hyper_request: Request<hyper::body::Incoming>,
    ) -> Result<Response<ResponseBody>, Infallible> {
        debug!("begin handle_request");

        let http_request = HttpRequest::new(connection_id, request_id, hyper_request);

        let result = self.handlers.handle(&http_request).await;

        debug!("end handle_request");
        Ok(result)
    }

    #[instrument(skip_all, fields(conn_id = connection.id().as_usize()))]
    async fn handle_connection<I: AsyncRead + AsyncWrite + Unpin + 'static>(
        self: Arc<Self>,
        stream: I,
        connection: ConnectionGuard,
        server_protocol: ServerProtocol,
    ) {
        info!("begin handle_connection");

        let service = service_fn(|hyper_request| {
            connection.increment_num_requests();

            let request_id = self.request_id_factory.new_request_id();

            Arc::clone(&self)
                .handle_request(connection.id(), request_id, hyper_request)
                .in_current_span()
        });

        if let Err(http_err) = match server_protocol {
            ServerProtocol::HTTP1 => {
                debug!("serving HTTP1 connection");
                HyperHTTP1Builder::new()
                    .serve_connection(stream, service)
                    .await
            }
            ServerProtocol::HTTP2 => {
                debug!("serving HTTP2 connection");
                HyperHTTP2Builder::new(TokioExecutor)
                    .serve_connection(stream, service)
                    .await
            }
        } {
            warn!(
                "Error while serving {:?} connection: {:?}",
                server_protocol, http_err,
            );
        }

        info!("end handle_connection");
    }
}

struct UnixServer {
    connection_handler: Arc<ConnectionHandler>,
    connection_tracker: &'static ConnectionTracker,
    server_configuration: &'static crate::config::ServerConfiguration,
}

impl UnixServer {
    async fn new(
        connection_handler: Arc<ConnectionHandler>,
        server_configuration: &'static crate::config::ServerConfiguration,
    ) -> Self {
        Self {
            connection_handler,
            connection_tracker: ConnectionTracker::instance().await,
            server_configuration,
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let path = self.server_configuration.bind_address();

        // do not fail on remove error, the path may not exist.
        let remove_result = tokio::fs::remove_file(path).await;
        debug!("remove_result = {:?}", remove_result);

        let unix_listener = UnixListener::bind(path)?;

        let local_addr = unix_listener.local_addr()?;
        info!("listening on unix {:?}", local_addr);

        loop {
            let (unix_stream, _remote_addr) = unix_listener.accept().await?;

            let connection = self
                .connection_tracker
                .add_connection(
                    *self.server_configuration.server_protocol(),
                    ServerSocketType::UNIX,
                )
                .await;

            tokio::task::spawn(Arc::clone(&self.connection_handler).handle_connection(
                unix_stream,
                connection,
                *self.server_configuration.server_protocol(),
            ));
        }
    }
}

struct TCPServer {
    connection_handler: Arc<ConnectionHandler>,
    connection_tracker: &'static ConnectionTracker,
    server_configuration: &'static crate::config::ServerConfiguration,
}

impl TCPServer {
    async fn new(
        connection_handler: Arc<ConnectionHandler>,
        server_configuration: &'static crate::config::ServerConfiguration,
    ) -> Self {
        Self {
            connection_handler,
            connection_tracker: ConnectionTracker::instance().await,
            server_configuration,
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let path = self.server_configuration.bind_address();

        let tcp_listener = TcpListener::bind(path).await?;

        let local_addr = tcp_listener.local_addr()?;
        info!("listening on tcp {:?}", local_addr);

        loop {
            let (tcp_stream, _remote_addr) = tcp_listener.accept().await?;

            if let Err(e) = tcp_stream.set_nodelay(true) {
                warn!("error setting tcp no delay {}", e);
                continue;
            };

            let connection = self
                .connection_tracker
                .add_connection(
                    *self.server_configuration.server_protocol(),
                    ServerSocketType::TCP,
                )
                .await;

            tokio::task::spawn(Arc::clone(&self.connection_handler).handle_connection(
                tcp_stream,
                connection,
                *self.server_configuration.server_protocol(),
            ));
        }
    }
}

pub struct Server {
    join_set: JoinSet<anyhow::Result<()>>,
}

impl Server {
    pub async fn new(handlers: Box<dyn RequestHandler>) -> Self {
        let request_id_factory = RequestIDFactory::new();
        let connection_handler = ConnectionHandler::new(handlers, request_id_factory);

        let mut join_set = JoinSet::new();

        let configuration = crate::config::instance();

        for server_configuration in configuration.server_configurations() {
            let connection_handler_clone = Arc::clone(&connection_handler);
            join_set.spawn(async move {
                match server_configuration.server_socket_type() {
                    ServerSocketType::TCP => {
                        let server =
                            TCPServer::new(connection_handler_clone, server_configuration).await;
                        server.run().await.context("TCP server run error")?;
                    }
                    ServerSocketType::UNIX => {
                        let server =
                            UnixServer::new(connection_handler_clone, server_configuration).await;
                        server.run().await.context("UNIX server run error")?;
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

#[derive(Clone)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}
