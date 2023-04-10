mod commands;
mod request_info;
mod route;
mod server_info;
mod utils;

use async_trait::async_trait;

use hyper::{http::Response, Body};

use crate::request::HttpRequest;

#[async_trait]
pub trait RequestHandler: Send + Sync {
    async fn handle(&self, request: &HttpRequest) -> Response<Body>;
}

pub async fn create_handlers() -> anyhow::Result<Box<dyn RequestHandler>> {
    let mut routes = Vec::new();

    routes.append(&mut commands::create_routes().await?);

    routes.append(&mut request_info::create_routes());

    routes.append(&mut server_info::create_routes().await);

    Ok(Box::new(route::Router::new(routes)?))
}
