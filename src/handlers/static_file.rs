use async_trait::async_trait;

use http_body_util::BodyExt;

use hyper::http::{Response, StatusCode};

use hyper_staticfile::{vfs::TokioFileOpener, ResolveResult, Resolver};

use tracing::{debug, warn};

use std::path::Path;

use tokio::time::Duration;

use crate::{
    handlers::{
        response_utils::build_status_code_response, HttpRequest, RequestHandler, ResponseBody,
    },
    response::CacheControl,
    static_file::StaticFileRulesService,
};

fn duration_to_u32_seconds(duration: Duration) -> u32 {
    duration.as_secs().try_into().unwrap_or_default()
}

#[derive(thiserror::Error, Debug)]
enum StaticFileHandlerError {
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
    client_error_page_cache_duration: Option<Duration>,
    static_file_rules_service: &'static StaticFileRulesService,
}

impl StaticFileHandler {
    fn new() -> anyhow::Result<Self> {
        let static_file_configuration = crate::config::instance().static_file_configuration();
        let root = Path::new(static_file_configuration.path());

        let mut resolver = Resolver::new(root);
        resolver.allowed_encodings.gzip = static_file_configuration.precompressed_gz();
        resolver.allowed_encodings.br = static_file_configuration.precompressed_br();

        debug!(
            "resolver.allowed_encodings = {:?}",
            resolver.allowed_encodings
        );

        Ok(Self {
            resolver,
            client_error_page_path: static_file_configuration.client_error_page_path(),
            client_error_page_cache_duration: *static_file_configuration
                .client_error_page_cache_duration(),
            static_file_rules_service: crate::static_file::rules_service_instance(),
        })
    }

    async fn build_client_error_page_response(
        &self,
    ) -> Result<Response<ResponseBody>, StaticFileHandlerError> {
        let client_error_page_request = hyper::http::Request::get(self.client_error_page_path)
            .body(())
            .unwrap();

        let resolve_result = self
            .resolver
            .resolve_request(&client_error_page_request)
            .await
            .map_err(StaticFileHandlerError::ClientErrorPageResolveRequest)?;

        let response = hyper_staticfile::ResponseBuilder::new()
            .request(&client_error_page_request)
            .cache_headers(
                self.client_error_page_cache_duration
                    .map(duration_to_u32_seconds),
            )
            .build(resolve_result)
            .map_err(StaticFileHandlerError::ClientErrorPageBuildResponse)?;

        let (mut parts, body) = response.into_parts();
        parts.status = StatusCode::NOT_FOUND;

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
            .resolve_request(request.hyper_request())
            .await
            .map_err(StaticFileHandlerError::ResolveRequest)?;

        debug!("resolve_result = {:?}", resolve_result);

        if matches!(
            resolve_result,
            ResolveResult::NotFound | ResolveResult::PermissionDenied
        ) || self.block_dot_paths(&resolve_result)
        {
            return self.build_client_error_page_response().await;
        }

        let cache_headers = self.build_cache_headers(&resolve_result);

        debug!("cache_headers = {:?}", cache_headers);

        let response = hyper_staticfile::ResponseBuilder::new()
            .request(request.hyper_request())
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

pub async fn create_default_route() -> anyhow::Result<Box<dyn RequestHandler>> {
    Ok(Box::new(StaticFileHandler::new()?))
}
