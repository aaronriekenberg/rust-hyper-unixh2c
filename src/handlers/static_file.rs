use anyhow::Context;

use async_trait::async_trait;

use bytes::Bytes;

use http_body_util::{combinators::BoxBody, BodyExt};

use hyper::{body::Body, http::Response};

use hyper_staticfile::vfs::TokioFileOpener;
use tracing::info;

use std::{convert::Infallible, io::Error, path::Path};

use crate::{
    handlers::{route::RouteInfo, utils::build_json_response, HttpRequest, RequestHandler},
    version::get_verison_info,
};

struct StaticFileHandler;

#[async_trait]
impl RequestHandler for StaticFileHandler {
    async fn handle(
        &self,
        request: &HttpRequest,
    ) -> Response<BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync + 'static>>> {
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
