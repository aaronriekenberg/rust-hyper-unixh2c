mod commands;
mod connection_info;
mod request_info;
mod response_utils;
mod route;
mod static_file;
mod time_utils;
mod version_info;

use async_trait::async_trait;

use hyper::http::Response;

use crate::{request::HttpRequest, response::ResponseBody};

#[async_trait]
pub trait RequestHandler: Send + Sync {
    async fn handle(&self, request: &HttpRequest) -> Response<ResponseBody>;
}

pub async fn create_handlers() -> anyhow::Result<Box<dyn RequestHandler>> {
    let mut routes = Vec::new();

    routes.append(&mut commands::create_routes().await?);

    routes.append(&mut connection_info::create_routes().await);

    routes.append(&mut request_info::create_routes());

    routes.append(&mut version_info::create_routes().await);

    let default_route = static_file::create_default_route().await;

    let router = Box::new(route::Router::new(routes, default_route)?);

    Ok(router)
}
