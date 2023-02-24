use hyper::{
    http::{Request, Response},
    server::conn::Http,
    service::service_fn,
    Body,
};

use tracing::{debug, info};

use tokio::net::{unix::SocketAddr, UnixListener, UnixStream};

use std::{convert::Infallible, os::fd::AsRawFd, sync::Arc};

use crate::{
    config::ServerProtocol,
    connection::{ConnectionID, ConnectionTracker},
    handlers::RequestHandler,
    request::{HttpRequest, RequestIDFactory},
};

pub struct Server {
    handlers: Box<dyn RequestHandler>,
    connection_tracker: Arc<ConnectionTracker>,
    request_id_factory: RequestIDFactory,
    server_protocol: ServerProtocol,
}

impl Server {
    pub fn new(
        connection_tracker: &Arc<ConnectionTracker>,
        handlers: Box<dyn RequestHandler>,
    ) -> Arc<Self> {
        Arc::new(Self {
            handlers,
            connection_tracker: Arc::clone(connection_tracker),
            request_id_factory: RequestIDFactory::new(),
            server_protocol: *crate::config::instance()
                .server_configuration()
                .server_protocol(),
        })
    }

    async fn handle_request(
        self: Arc<Self>,
        connection_id: ConnectionID,
        hyper_request: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        let request_id = self.request_id_factory.new_request_id();

        let http_request = HttpRequest::new(connection_id, request_id, hyper_request);

        let result = self.handlers.handle(&http_request).await;
        Ok(result)
    }

    fn handle_connection(self: Arc<Self>, unix_stream: UnixStream, remote_addr: SocketAddr) {
        tokio::task::spawn(async move {
            let fd = unix_stream.as_raw_fd();

            let connection = self
                .connection_tracker
                .add_connection(self.server_protocol)
                .await;

            let connection_id = connection.connection_id();

            info!(
                "got connection from {:?} connection_id = {:?} fd = {}",
                remote_addr, connection_id, fd,
            );

            let service = service_fn(|request| {
                let self_clone = Arc::clone(&self);

                connection.increment_num_requests();

                async move { self_clone.handle_request(connection_id, request).await }
            });

            let mut http = Http::new();

            match self.server_protocol {
                ServerProtocol::HTTP1 => http.http1_only(true),
                ServerProtocol::HTTP2 => http.http2_only(true),
            };

            if let Err(http_err) = http.serve_connection(unix_stream, service).await {
                info!("Error while serving HTTP connection: {:?}", http_err);
            }

            info!(
                "end connection from {:?} connection_id = {:?} fd = {}",
                remote_addr, connection_id, fd,
            );

            drop(connection);
        });
    }

    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        let path = crate::config::instance()
            .server_configuration()
            .bind_address();

        // do not fail on remove error, the path may not exist.
        let remove_result = tokio::fs::remove_file(path).await;
        debug!("remove_result = {:?}", remove_result);

        let unix_listener = UnixListener::bind(&path)?;
        let fd = unix_listener.as_raw_fd();

        let local_addr = unix_listener.local_addr()?;
        info!("listening on {:?} fd = {}", local_addr, fd);

        loop {
            let (unix_stream, remote_addr) = unix_listener.accept().await?;

            Arc::clone(&self).handle_connection(unix_stream, remote_addr);
        }
    }
}
