use async_trait::async_trait;

use http_body_util::BodyExt;

use hyper::http::{Response, StatusCode};

use hyper_staticfile::{vfs::TokioFileOpener, Resolver};

use tracing::{info, warn};

use std::path::Path;

use crate::{
    handlers::{utils::build_status_code_response, HttpRequest, RequestHandler, ResponseBody},
    response::CacheControl,
};

struct StaticFileHandler {
    resolver: Resolver<TokioFileOpener>,
}

impl StaticFileHandler {
    fn new() -> Self {
        let config = crate::config::instance();
        let root = Path::new(config.static_file_configuration().path());

        Self {
            resolver: hyper_staticfile::Resolver::new(root),
        }
    }
}

#[async_trait]
impl RequestHandler for StaticFileHandler {
    async fn handle(&self, request: &HttpRequest) -> Response<ResponseBody> {
        info!("handle_static_file request = {:?}", request);

        let resolve_result = match self.resolver.resolve_request(request.hyper_request()).await {
            Ok(resolve_result) => resolve_result,
            Err(e) => {
                warn!("resolve_request error e = {}", e);
                return build_status_code_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    CacheControl::NoCache,
                );
            }
        };

        info!("resolve_result = {:?}", resolve_result);

        let response = match hyper_staticfile::ResponseBuilder::new()
            .request(request.hyper_request())
            .build(resolve_result)
        {
            Ok(response) => response,
            Err(e) => {
                warn!("ResponseBuilder.build error e = {}", e);
                return build_status_code_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    CacheControl::NoCache,
                );
            }
        };

        let (parts, body) = response.into_parts();

        let boxed_body = body.map_err(|e| e.into()).boxed();

        Response::from_parts(parts, boxed_body)
    }
}

pub async fn create_default_route() -> Box<dyn RequestHandler> {
    Box::new(StaticFileHandler::new())
}
