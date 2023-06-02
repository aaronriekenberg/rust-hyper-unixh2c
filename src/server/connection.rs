use hyper::{
    http::{Request, Response},
    server::conn::http1::Builder as HyperHTTP1Builder,
    server::conn::http2::Builder as HyperHTTP2Builder,
    service::service_fn,
};

use tracing::{debug, info, instrument, warn, Instrument};

use tokio::io::{AsyncRead, AsyncWrite};

use std::{convert::Infallible, sync::Arc};

use crate::{
    config::ServerProtocol,
    connection::{ConnectionGuard, ConnectionID},
    handlers::RequestHandler,
    request::{HttpRequest, RequestID, RequestIDFactory},
    response::ResponseBody,
};
pub struct ConnectionHandler {
    handlers: Box<dyn RequestHandler>,
    request_id_factory: RequestIDFactory,
}

impl ConnectionHandler {
    pub fn new(
        handlers: Box<dyn RequestHandler>,
        request_id_factory: RequestIDFactory,
    ) -> Arc<Self> {
        Arc::new(Self {
            handlers,
            request_id_factory,
        })
    }

    #[instrument(skip_all, fields(req_id = request_id.as_usize()))]
    pub async fn handle_request(
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
    pub async fn handle_connection<I: AsyncRead + AsyncWrite + Unpin + 'static>(
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
            ServerProtocol::Http1 => {
                debug!("serving HTTP1 connection");
                HyperHTTP1Builder::new()
                    .serve_connection(stream, service)
                    .await
            }
            ServerProtocol::Http2 => {
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
