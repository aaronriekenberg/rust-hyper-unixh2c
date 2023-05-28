use async_trait::async_trait;

use http_body_util::BodyExt;

use hyper::http::Response;

use tracing::info;

use std::path::Path;

use crate::handlers::{HttpRequest, RequestHandler, ResponseBody};

struct StaticFileHandler;

#[async_trait]
impl RequestHandler for StaticFileHandler {
    async fn handle(&self, request: &HttpRequest) -> Response<ResponseBody> {
        info!("handle_static_file request = {:?}", request);

        let root = Path::new("/Users/aaron/aaronr.digital");

        let resolver = hyper_staticfile::Resolver::new(root);

        let resolve_result = resolver
            .resolve_request(request.hyper_request())
            .await
            .unwrap();

        info!("resolve_result = {:?}", resolve_result);

        let response = hyper_staticfile::ResponseBuilder::new()
            .request(request.hyper_request())
            .build(resolve_result)
            .unwrap();

        let (parts, body) = response.into_parts();

        let boxed_body = body.map_err(|e| e.into()).boxed();

        Response::from_parts(parts, boxed_body)
    }
}

pub async fn create_default_route() -> Box<dyn RequestHandler> {
    Box::new(StaticFileHandler)
}
