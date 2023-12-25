use async_trait::async_trait;

use http_body_util::BodyExt;

use hyper::http::{header, Request as HyperHttpRequest, Response, StatusCode};

use hyper_staticfile::{vfs::TokioFileOpener, ResolveResult, Resolver};

use tracing::{debug, warn};

use tokio::time::Duration;

use crate::{
    handlers::{
        response_utils::build_status_code_response, HttpRequest, RequestHandler, ResponseBody,
    },
    response::CacheControl,
    static_file::StaticFileRulesService,
};

#[derive(thiserror::Error, Debug)]
enum StaticFileHandlerError {
    #[error("client error page build request error: {0}")]
    ClientErrorPageBuildRequest(hyper::http::Error),

    #[error("client error page resolve error: {0}")]
    ClientErrorPageResolveRequest(std::io::Error),

    #[error("client error page build response error: {0}")]
    ClientErrorPageBuildResponse(hyper::http::Error),

    #[error("resolve error: {0}")]
    ResolveRequest(std::io::Error),

    #[error("build response error: {0}")]
    BuildResponse(hyper::http::Error),
}

struct StaticFileHandler {
    resolver: Resolver<TokioFileOpener>,
    client_error_page_path: &'static str,
    static_file_rules_service: &'static StaticFileRulesService,
}

impl StaticFileHandler {
    fn new() -> Self {
        let static_file_configuration = &crate::config::instance().static_file_configuration;

        let mut resolver = Resolver::new(&static_file_configuration.root);
        resolver.allowed_encodings.gzip = static_file_configuration.precompressed.gz;
        resolver.allowed_encodings.br = static_file_configuration.precompressed.br;

        debug!(
            "resolver.allowed_encodings = {:?}",
            resolver.allowed_encodings
        );

        Self {
            resolver,
            client_error_page_path: &static_file_configuration.client_error_page_path,
            static_file_rules_service: crate::static_file::rules_service_instance(),
        }
    }

    async fn build_client_error_page_response(
        &self,
        original_request: &HttpRequest,
        status_code: StatusCode,
    ) -> Result<Response<ResponseBody>, StaticFileHandlerError> {
        let mut client_error_page_request = HyperHttpRequest::get(self.client_error_page_path);

        // copy ACCEPT_ENCODING header from original request
        // so we can try to use gz/bz client error page if possible.
        if let Some(accept_encoding_header_value) = original_request
            .hyper_request
            .headers()
            .get(header::ACCEPT_ENCODING)
        {
            client_error_page_request = client_error_page_request
                .header(header::ACCEPT_ENCODING, accept_encoding_header_value);
        }

        let client_error_page_request = client_error_page_request
            .body(())
            .map_err(StaticFileHandlerError::ClientErrorPageBuildRequest)?;

        let resolve_result = self
            .resolver
            .resolve_request(&client_error_page_request)
            .await
            .map_err(StaticFileHandlerError::ClientErrorPageResolveRequest)?;

        let response = hyper_staticfile::ResponseBuilder::new()
            .request(&client_error_page_request)
            .cache_headers(self.build_cache_headers(&resolve_result))
            .build(resolve_result)
            .map_err(StaticFileHandlerError::ClientErrorPageBuildResponse)?;

        let (mut parts, body) = response.into_parts();
        parts.status = status_code;

        let boxed_body = body.map_err(|e| e.into()).boxed();

        Ok(Response::from_parts(parts, boxed_body))
    }

    fn block_dot_paths(&self, resolve_result: &ResolveResult) -> bool {
        let str_path_option = match resolve_result {
            ResolveResult::Found(resolved_file) => resolved_file.path.to_str(),
            ResolveResult::IsDirectory { redirect_to } => Some(redirect_to.as_str()),
            _ => None,
        };

        if let Some(str_path) = str_path_option {
            debug!("str_path = {}", str_path);
            if str_path.starts_with('.') || str_path.contains("/.") {
                warn!("blocking request for dot file path = {:?}", str_path);
                return true;
            }
        };

        false
    }

    fn build_cache_headers(&self, resolve_result: &ResolveResult) -> Option<u32> {
        fn duration_to_u32_seconds(duration: Duration) -> u32 {
            duration.as_secs().try_into().unwrap_or_default()
        }

        match resolve_result {
            ResolveResult::Found(resolved_file) => self
                .static_file_rules_service
                .build_cache_header(resolved_file)
                .map(duration_to_u32_seconds),
            _ => None,
        }
    }

    async fn try_handle(
        &self,
        request: &HttpRequest,
    ) -> Result<Response<ResponseBody>, StaticFileHandlerError> {
        debug!("StaticFileHandler::try_handle request = {:?}", request);

        let resolve_result = self
            .resolver
            .resolve_request(&request.hyper_request)
            .await
            .map_err(StaticFileHandlerError::ResolveRequest)?;

        debug!("resolve_result = {:?}", resolve_result);

        if matches!(resolve_result, ResolveResult::MethodNotMatched) {
            return self
                .build_client_error_page_response(request, StatusCode::BAD_REQUEST)
                .await;
        } else if matches!(resolve_result, ResolveResult::NotFound) {
            return self
                .build_client_error_page_response(request, StatusCode::NOT_FOUND)
                .await;
        } else if matches!(resolve_result, ResolveResult::PermissionDenied)
            || self.block_dot_paths(&resolve_result)
        {
            return self
                .build_client_error_page_response(request, StatusCode::FORBIDDEN)
                .await;
        }

        let cache_headers = self.build_cache_headers(&resolve_result);

        debug!("cache_headers = {:?}", cache_headers);

        let response = hyper_staticfile::ResponseBuilder::new()
            .request(&request.hyper_request)
            .cache_headers(cache_headers)
            .build(resolve_result)
            .map_err(StaticFileHandlerError::BuildResponse)?;

        let (parts, body) = response.into_parts();

        let boxed_body = body.map_err(|e| e.into()).boxed();

        Ok(Response::from_parts(parts, boxed_body))
    }
}

#[async_trait]
impl RequestHandler for StaticFileHandler {
    async fn handle(&self, request: &HttpRequest) -> Response<ResponseBody> {
        match self.try_handle(request).await {
            Ok(response) => response,
            Err(e) => {
                warn!("StaticFileHandler::try_handle error: {}", e);
                build_status_code_response(StatusCode::INTERNAL_SERVER_ERROR, CacheControl::NoCache)
            }
        }
    }
}

pub fn create_default_route() -> Box<dyn RequestHandler> {
    Box::new(StaticFileHandler::new())
}
