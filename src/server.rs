use hyper::http::{Request, Response};
use hyper::{server::conn::Http, service::service_fn, Body};

use tracing::{debug, info};

use tokio::net::UnixListener;

use std::convert::Infallible;
use std::sync::Arc;

use crate::handlers::RequestHandler;

pub async fn run_server(handlers: Arc<dyn RequestHandler>) -> anyhow::Result<()> {
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
        let handlers_clone = Arc::clone(&handlers);
        let (unix_stream, remote_addr) = unix_listener.accept().await?;
        tokio::task::spawn(async move {
            info!("got connection from {:?}", remote_addr);

            let service = service_fn(|req| {
                let handlers_service_clone = Arc::clone(&handlers_clone);
                invoke_handlers(handlers_service_clone, req)
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
}

async fn invoke_handlers(
    handlers: Arc<dyn RequestHandler>,
    request: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    Ok(handlers.handle(request).await)
}
