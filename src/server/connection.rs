use hyper::{
    http::{Request, Response},
    server::conn::http1::Builder as HyperHTTP1Builder,
    server::conn::http2::Builder as HyperHTTP2Builder,
    service::service_fn,
};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Duration,
};

use tracing::{debug, info, instrument, warn, Instrument};

use std::{convert::Infallible, pin::Pin, sync::Arc};

use crate::{
    config::ServerProtocol,
    connection::{ConnectionGuard, ConnectionID},
    handlers::RequestHandler,
    request::{HttpRequest, RequestID, RequestIDFactory},
    response::ResponseBody,
};

const CONNECTION_MAX_LIFETIME_DURATION: Duration = Duration::from_secs(120);

const CONNECTION_GRACEFUL_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);

const CONNECTION_TIMEOUT_DURATIONS: &[Duration] = &[
    CONNECTION_MAX_LIFETIME_DURATION,
    CONNECTION_GRACEFUL_SHUTDOWN_DURATION,
];

pub struct ConnectionHandler {
    request_handler: Box<dyn RequestHandler>,
    request_id_factory: RequestIDFactory,
}

impl ConnectionHandler {
    pub fn new(
        request_handler: Box<dyn RequestHandler>,
        request_id_factory: RequestIDFactory,
    ) -> Arc<Self> {
        Arc::new(Self {
            request_handler,
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

        let result = self.request_handler.handle(&http_request).await;

        debug!("end handle_request");
        Ok(result)
    }

    #[instrument(skip_all, fields(
        conn_id = connection.id().as_usize(),
        sock = ?connection.server_socket_type(),
        proto = ?connection.server_protocol(),
    ))]
    pub async fn handle_connection(
        self: Arc<Self>,
        stream: impl AsyncRead + AsyncWrite + Unpin + 'static,
        connection: ConnectionGuard,
    ) {
        info!("begin handle_connection");

        match connection.server_protocol() {
            ServerProtocol::Http1 => self.handle_h1_connection(stream, connection).await,
            ServerProtocol::Http2 => self.handle_h2_connection(stream, connection).await,
        };

        info!("end handle_connection");
    }

    async fn handle_h1_connection(
        self: Arc<Self>,
        stream: impl AsyncRead + AsyncWrite + Unpin + 'static,
        connection: ConnectionGuard,
    ) {
        let service = service_fn(|hyper_request| {
            connection.increment_num_requests();

            let request_id = self.request_id_factory.new_request_id();

            Arc::clone(&self)
                .handle_request(connection.id(), request_id, hyper_request)
                .in_current_span()
        });

        debug!("serving HTTP1 connection");
        let mut conn = HyperHTTP1Builder::new().serve_connection(stream, service);

        let mut conn = Pin::new(&mut conn);

        for (iter, sleep_duration) in CONNECTION_TIMEOUT_DURATIONS.iter().enumerate() {
            debug!("iter = {} sleep_duration = {:?}", iter, sleep_duration);
            tokio::select! {
                res = conn.as_mut() => {
                    match res {
                        Ok(()) => debug!("after polling conn, no error"),
                        Err(e) =>  warn!("error serving connection: {:?}", e),
                    };
                    break;
                }
                _ = tokio::time::sleep(*sleep_duration) => {
                    info!("iter = {} got timeout_interval, calling conn.graceful_shutdown", iter);
                    conn.as_mut().graceful_shutdown();
                }
            }
        }
    }

    async fn handle_h2_connection(
        self: Arc<Self>,
        stream: impl AsyncRead + AsyncWrite + Unpin + 'static,
        connection: ConnectionGuard,
    ) {
        let service = service_fn(|hyper_request| {
            connection.increment_num_requests();

            let request_id = self.request_id_factory.new_request_id();

            Arc::clone(&self)
                .handle_request(connection.id(), request_id, hyper_request)
                .in_current_span()
        });

        debug!("serving HTTP2 connection");
        let mut conn = HyperHTTP2Builder::new(TokioExecutor).serve_connection(stream, service);

        let mut conn = Pin::new(&mut conn);

        for (iter, sleep_duration) in CONNECTION_TIMEOUT_DURATIONS.iter().enumerate() {
            debug!("iter = {} sleep_duration = {:?}", iter, sleep_duration);
            tokio::select! {
                res = conn.as_mut() => {
                    match res {
                        Ok(()) => debug!("after polling conn, no error"),
                        Err(e) =>  warn!("error serving connection: {:?}", e),
                    };
                    break;
                }
                _ = tokio::time::sleep(*sleep_duration) => {
                    info!("iter = {} got timeout_interval, calling conn.graceful_shutdown", iter);
                    conn.as_mut().graceful_shutdown();
                }
            }
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
