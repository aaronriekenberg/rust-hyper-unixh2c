mod request_info;
mod route;
mod utils;

use async_trait::async_trait;

use hyper::{
    http::{Request, Response},
    Body,
};

use std::sync::Arc;

#[async_trait]
pub trait RequestHandler: Send + Sync {
    async fn handle(&self, request: Request<Body>) -> Response<Body>;
}

pub fn create_handlers() -> anyhow::Result<Arc<dyn RequestHandler>> {
    let mut routes = Vec::new();

    routes.append(&mut request_info::create_routes());

    Ok(Arc::new(route::Router::new(routes)?))
}
