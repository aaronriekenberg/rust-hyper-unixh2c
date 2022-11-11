use hyper::{
    http::{Request, Response},
    server::conn::Http,
    service::service_fn,
    Body,
};

use tracing::{debug, info};

use tokio::net::{unix::SocketAddr, UnixListener, UnixStream};

use std::{convert::Infallible, sync::Arc};

use crate::handlers::RequestHandler;

pub struct Server {
    handlers: Box<dyn RequestHandler>,
}

impl Server {
    pub fn new(handlers: Box<dyn RequestHandler>) -> Arc<Self> {
        Arc::new(Self { handlers })
    }

    async fn handle_request(
        self: Arc<Self>,
        request: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        let result = self.handlers.handle(request).await;
        Ok(result)
    }

    fn handle_connection(self: Arc<Self>, unix_stream: UnixStream, remote_addr: SocketAddr) {
        tokio::task::spawn(async move {
            info!("got connection from {:?}", remote_addr);

            let service = service_fn(move |request| {
                let self_clone = Arc::clone(&self);

                async move { self_clone.handle_request(request).await }
            });

            if let Err(http_err) = Http::new()
                .http2_only(true)
                .serve_connection(unix_stream, service)
                .await
            {
                info!("Error while serving HTTP connection: {:?}", http_err);
            }

            info!("end connection from {:?}", remote_addr);
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

        let local_addr = unix_listener.local_addr()?;
        info!("listening on {:?}", local_addr);

        loop {
            let (unix_stream, remote_addr) = unix_listener.accept().await?;

            Arc::clone(&self).handle_connection(unix_stream, remote_addr);
        }
    }
}
