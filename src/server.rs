use hyper::{
    http::{Request, Response},
    server::conn::Http,
    service::service_fn,
    Body,
};

use tracing::{debug, info, instrument, Instrument, Span};

use tokio::net::{UnixListener, UnixStream};

use std::{convert::Infallible, sync::Arc};

use crate::{
    config::{ServerConfiguration, ServerProtocol},
    connection::{ConnectionGuard, ConnectionID, ConnectionTracker},
    handlers::RequestHandler,
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

    #[instrument(skip_all, fields(request_id = request_id.0))]
    async fn handle_request(
        self: Arc<Self>,
        connection_id: ConnectionID,
        request_id: RequestID,
        hyper_request: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        debug!("begin handle_request");

        let http_request = HttpRequest::new(connection_id, request_id, hyper_request);

        let result = self.handlers.handle(&http_request).await;

        debug!("end handle_request");
        Ok(result)
    }

    #[instrument(skip_all, fields(connection_id = connection.id().0))]
    async fn handle_connection(
        self: Arc<Self>,
        unix_stream: UnixStream,
        connection: ConnectionGuard,
    ) {
        info!("begin handle_connection");

        let span = Span::current();

        let service = service_fn(|hyper_request| {
            let self_clone = Arc::clone(&self);

            connection.increment_num_requests();

            let connection_id = connection.id();

            let request_id = self.request_id_factory.new_request_id();

            let span_clone = span.clone();

            async move {
                self_clone
                    .handle_request(connection_id, request_id, hyper_request)
                    .instrument(span_clone)
                    .await
            }
        });

        let mut http = Http::new();

        match self.server_configuration.server_protocol() {
            ServerProtocol::HTTP1 => http.http1_only(true),
            ServerProtocol::HTTP2 => http.http2_only(true),
        };

        if let Err(http_err) = http.serve_connection(unix_stream, service).await {
            info!("Error while serving HTTP connection: {:?}", http_err);
        }

        info!("end handle_connection");
    }

    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        let path = self.server_configuration.bind_address();

        // do not fail on remove error, the path may not exist.
        let remove_result = tokio::fs::remove_file(path).await;
        debug!("remove_result = {:?}", remove_result);

        let unix_listener = UnixListener::bind(&path)?;

        let local_addr = unix_listener.local_addr()?;
        info!("listening on {:?}", local_addr);

        loop {
            let (unix_stream, _remote_addr) = unix_listener.accept().await?;

            let connection = self
                .connection_tracker
                .add_connection(*self.server_configuration.server_protocol())
                .await;

            let self_clone = Arc::clone(&self);
            tokio::spawn(
                async move { self_clone.handle_connection(unix_stream, connection).await },
            );
        }
    }
}
