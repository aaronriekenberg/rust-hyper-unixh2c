use hyper::{
    http::{Request, Response},
    server::conn::http1::Builder as HyperHTTP1Builder,
    server::conn::http2::Builder as HyperHTTP2Builder,
    service::service_fn,
};

use hyper_util::rt::TokioExecutor;

use tokio::{pin, time::Duration};

use tracing::{debug, info, instrument, warn, Instrument};

use std::{convert::Infallible, sync::Arc};

use crate::{
    config::ServerProtocol,
    connection::{ConnectionGuard, ConnectionID},
    handlers::RequestHandler,
    request::{HttpRequest, RequestID, RequestIDFactory},
    response::ResponseBody,
    server::{h1h2conn::HyperH1OrH2Connection, utils::HyperReadWrite},
};

pub struct ConnectionHandler {
    request_handler: Box<dyn RequestHandler>,
    request_id_factory: RequestIDFactory,
    connection_timeout_durations: Vec<Duration>,
    tokio_executor: TokioExecutor,
}

impl ConnectionHandler {
    pub fn new(
        request_handler: Box<dyn RequestHandler>,
        request_id_factory: RequestIDFactory,
    ) -> Arc<Self> {
        let server_configuration = crate::config::instance().server_configuration();

        let connection_timeout_durations = vec![
            server_configuration.connection_max_lifetime(),
            server_configuration.connection_graceful_shutdown_timeout(),
        ];

        debug!(
            "connection_timeout_durations = {:?}",
            connection_timeout_durations
        );

        Arc::new(Self {
            request_handler,
            request_id_factory,
            connection_timeout_durations,
            tokio_executor: TokioExecutor::new(),
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

        let result = self.request_handler.handle(&http_request).await;

        debug!("end handle_request");
        Ok(result)
    }

    #[instrument(skip_all, fields(
        conn_id = connection.id().as_usize(),
        sock = ?connection.server_socket_type(),
        proto = ?connection.server_protocol(),
    ))]
    async fn handle_connection(
        self: Arc<Self>,
        stream: impl HyperReadWrite,
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

        let hyper_conn = match connection.server_protocol() {
            ServerProtocol::Http1 => {
                let conn = HyperHTTP1Builder::new().serve_connection(stream, service);
                HyperH1OrH2Connection::H1(conn)
            }
            ServerProtocol::Http2 => {
                let conn = HyperHTTP2Builder::new(self.tokio_executor.clone())
                    .serve_connection(stream, service);
                HyperH1OrH2Connection::H2(conn)
            }
        };
        pin!(hyper_conn);

        for (iter, sleep_duration) in self.connection_timeout_durations.iter().enumerate() {
            debug!("iter = {} sleep_duration = {:?}", iter, sleep_duration);
            tokio::select! {
                res = hyper_conn.as_mut() => {
                    match res {
                        Ok(()) => debug!("after polling conn, no error"),
                        Err(e) =>  warn!("error serving connection: {:?}", e),
                    };
                    break;
                }
                _ = tokio::time::sleep(*sleep_duration) => {
                    info!("iter = {} got timeout_interval, calling conn.graceful_shutdown", iter);
                    hyper_conn.as_mut().graceful_shutdown();
                }
            }
        }

        info!(
            "end handle_connection num_requests = {}",
            connection.num_requests()
        );
    }

    pub fn start_connection_handler(
        self: &Arc<Self>,
        stream: impl HyperReadWrite,
        connection: ConnectionGuard,
    ) {
        tokio::spawn(Arc::clone(self).handle_connection(stream, connection));
    }
}
