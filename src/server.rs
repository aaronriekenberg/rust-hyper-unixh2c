use hyper::{
    http::{Request, Response},
    server::conn::http1::Builder as HyperHTTP1Builder,
    server::conn::http2::Builder as HyperHTTP2Builder,
    service::service_fn,
};

use tracing::{debug, info, instrument, warn, Instrument};

use tokio::net::{UnixListener, UnixStream};

use std::{convert::Infallible, sync::Arc};

use crate::{
    config::{ServerConfiguration, ServerProtocol},
    connection::{ConnectionGuard, ConnectionID, ConnectionTracker},
    handlers::{RequestHandler, ResponseBody},
    request::{HttpRequest, RequestID, RequestIDFactory},
};

pub struct Server {
    handlers: Box<dyn RequestHandler>,
    connection_tracker: &'static ConnectionTracker,
    request_id_factory: RequestIDFactory,
    server_configuration: &'static ServerConfiguration,
}

impl Server {
    pub async fn new(handlers: Box<dyn RequestHandler>) -> Arc<Self> {
        Arc::new(Self {
            handlers,
            connection_tracker: ConnectionTracker::instance().await,
            request_id_factory: RequestIDFactory::new(),
            server_configuration: crate::config::instance().server_configuration(),
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
    async fn handle_connection(
        self: Arc<Self>,
        unix_stream: UnixStream,
        connection: ConnectionGuard,
    ) {
        info!("begin handle_connection");

        let service = service_fn(|hyper_request| {
            connection.increment_num_requests();

            let request_id = self.request_id_factory.new_request_id();

            Arc::clone(&self)
                .handle_request(connection.id(), request_id, hyper_request)
                .in_current_span()
        });

        if let Err(http_err) = match self.server_configuration.server_protocol() {
            ServerProtocol::HTTP1 => {
                debug!("serving HTTP1 connection");
                HyperHTTP1Builder::new()
                    .serve_connection(unix_stream, service)
                    .await
            }
            ServerProtocol::HTTP2 => {
                debug!("serving HTTP2 connection");
                HyperHTTP2Builder::new(TokioExecutor)
                    .serve_connection(unix_stream, service)
                    .await
            }
        } {
            warn!(
                "Error while serving {:?} connection: {:?}",
                self.server_configuration.server_protocol(),
                http_err,
            );
        }

        info!("end handle_connection");
    }

    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        let path = self.server_configuration.bind_address();

        // do not fail on remove error, the path may not exist.
        let remove_result = tokio::fs::remove_file(path).await;
        debug!("remove_result = {:?}", remove_result);

        let unix_listener = UnixListener::bind(path)?;

        let local_addr = unix_listener.local_addr()?;
        info!("listening on {:?}", local_addr);

        loop {
            let (unix_stream, _remote_addr) = unix_listener.accept().await?;

            let connection = self
                .connection_tracker
                .add_connection(*self.server_configuration.server_protocol())
                .await;

            tokio::task::spawn(Arc::clone(&self).handle_connection(unix_stream, connection));
        }
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
