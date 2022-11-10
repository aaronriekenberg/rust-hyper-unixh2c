use hyper::http::{Request, Response, StatusCode};
use hyper::{server::conn::Http, service::service_fn, Body};

use tokio::net::UnixListener;

use tracing::{debug, info};

use std::convert::Infallible;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let path = "./socket";

    // do not fail on remove error, the path may not exist.
    let remove_result = tokio::fs::remove_file(path).await;
    debug!("remove_result = {:?}", remove_result);

    let unix_listener = UnixListener::bind(&path)?;

    let local_addr = unix_listener.local_addr()?;
    info!("listening on {:?}", local_addr);

    loop {
        let (unix_stream, remote_addr) = unix_listener.accept().await?;
        tokio::task::spawn(async move {
            info!("got connection from {:?}", remote_addr);
            if let Err(http_err) = Http::new()
                .http2_only(true)
                .serve_connection(unix_stream, service_fn(hello))
                .await
            {
                info!("Error while serving HTTP connection: {:?}", http_err);
            }
            info!("end connection from {:?}", remote_addr);
        });
    }
}

async fn hello(request: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("request = {:?}", request);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("X-Custom-Foo", "Bar")
        .body(Body::from("Hello World!"))
        .unwrap())
}
