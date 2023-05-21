use std::path::PathBuf;

use async_trait::async_trait;

use hyper::{Body, Method, Response};

use tracing::{debug, info};

use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path},
};

use crate::{
    handlers::{route::RouteInfo, utils::build_json_response, HttpRequest, RequestHandler},
    version::get_verison_info,
};

struct StaticFileHandler;

#[async_trait]
impl RequestHandler for StaticFileHandler {
    async fn handle(&self, request: &HttpRequest) -> Response<Body> {
        info!("handle_static_file request = {:?}", request);

        let root = Path::new("/Users/aaron/aaronr.digital");

        let resolve_result = hyper_staticfile::resolve(&root, request.hyper_request())
            .await
            .unwrap();

        info!("resolve_result = {:?}", resolve_result);

        let response = hyper_staticfile::ResponseBuilder::new()
            .request(request.hyper_request())
            .build(resolve_result)
            .unwrap();

        response
    }
}

pub async fn create_default_route() -> Box<dyn RequestHandler>{
     Box::new(StaticFileHandler)
}
