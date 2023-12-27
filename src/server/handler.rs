use hyper::{
    http::{Request, Response},
    service::service_fn,
};

use hyper_util::{rt::TokioExecutor, server::conn::auto::Builder as HyperConnAutoBuilder};

use tokio::{
    pin,
    time::{Duration, Instant},
};

use tracing::{debug, info, instrument, warn, Instrument};

use std::{convert::Infallible, sync::Arc};

use crate::{
    connection::{ConnectionGuard, ConnectionID},
    handlers::RequestHandler,
    request::{HttpRequest, RequestID, RequestIDFactory},
    response::ResponseBody,
    server::HyperReadWrite,
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
        let server_configuration = &crate::config::instance().server_configuration;

        let connection_timeout_durations = vec![
            server_configuration.connection.max_lifetime,
            server_configuration.connection.graceful_shutdown_timeout,
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

    #[instrument(
        name = "request",
        skip_all,
        fields(
            id = request_id.as_usize(),
            method = %hyper_request.method(),
            uri = %hyper_request.uri(),
            micros,
            status,
        )
    )]
    async fn handle_request(
        self: Arc<Self>,
        connection_id: ConnectionID,
        request_id: RequestID,
        hyper_request: Request<hyper::body::Incoming>,
    ) -> Result<Response<ResponseBody>, Infallible> {
        let start_time = Instant::now();

        let http_request = HttpRequest::new(connection_id, request_id, hyper_request);

        let result = self.request_handler.handle(&http_request).await;

        let duration = Instant::now() - start_time;

        let status = result.status();

        tracing::Span::current()
            .record("micros", duration.as_micros())
            .record("status", status.as_u16());

        if status.is_informational() || status.is_success() || status.is_redirection() {
            debug!("request complete");
        } else if status.is_client_error() {
            info!("request complete");
        } else {
            warn!("request complete");
        };

        Ok(result)
    }

    #[instrument(
        name = "conn",
        skip_all,
        fields(
            id = connection.id.as_usize(),
            sock = ?connection.server_socket_type,
        )
    )]
    async fn handle_connection(
        self: Arc<Self>,
        stream: impl HyperReadWrite,
        connection: ConnectionGuard,
    ) {
        debug!("begin handle_connection");

        let service = service_fn(|hyper_request| {
            connection.increment_num_requests();

            let request_id = self.request_id_factory.new_request_id();

            Arc::clone(&self)
                .handle_request(connection.id, request_id, hyper_request)
                .in_current_span()
        });

        let builder = HyperConnAutoBuilder::new(self.tokio_executor.clone());

        let hyper_conn = builder.serve_connection(stream, service);
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

        debug!(
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
