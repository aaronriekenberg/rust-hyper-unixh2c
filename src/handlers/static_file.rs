use anyhow::Context;
use async_trait::async_trait;

use http_body_util::BodyExt;

use hyper::http::{Response, StatusCode};

use hyper_staticfile::{vfs::TokioFileOpener, ResolveResult, Resolver};

use tracing::{debug, warn};

use std::{path::Path, time::SystemTime};

use tokio::time::Duration;

use crate::{
    handlers::{
        response_utils::build_status_code_response, HttpRequest, RequestHandler, ResponseBody,
    },
    response::CacheControl,
};

const DEFAULT_CACHE_DURATION_SECONDS: u32 = 60 * 60;

const VNSTAT_PNG_CACHE_DURATION: Duration = Duration::from_secs(15 * 60);

const CLIENT_ERROR_PAGE_CACHE_DURATION_SECONDS: u32 = 5 * 60;

struct StaticFileHandler {
    resolver: Resolver<TokioFileOpener>,
    client_error_page_path: &'static str,
}

impl StaticFileHandler {
    fn new() -> Self {
        let static_file_configuration = crate::config::instance().static_file_configuration();
        let root = Path::new(static_file_configuration.path());

        let mut resolver = Resolver::new(root);
        resolver.allowed_encodings.gzip = static_file_configuration.precompressed_gz();
        resolver.allowed_encodings.br = static_file_configuration.precompressed_br();

        debug!(
            "resolver.allowed_encodings = {:?}",
            resolver.allowed_encodings
        );

        Self {
            resolver,
            client_error_page_path: static_file_configuration.client_error_page_path(),
        }
    }

    async fn build_client_error_page_response(&self) -> anyhow::Result<Response<ResponseBody>> {
        let client_error_page_request = hyper::http::Request::get(self.client_error_page_path)
            .body(())
            .unwrap();

        let resolve_result = self
            .resolver
            .resolve_request(&client_error_page_request)
            .await
            .context("build_client_error_page_response: resolve_request error")?;

        let response = hyper_staticfile::ResponseBuilder::new()
            .request(&client_error_page_request)
            .cache_headers(Some(CLIENT_ERROR_PAGE_CACHE_DURATION_SECONDS))
            .build(resolve_result)
            .context("build_client_error_page_response: ResponseBuilder.build error")?;

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
            ResolveResult::Found(resolved_file) => {
                debug!("resolved_file.path = {:?}", resolved_file.path,);

                let str_path = resolved_file.path.to_str().unwrap_or_default();

                if !(str_path.contains("vnstat/") && str_path.ends_with(".png")) {
                    Some(DEFAULT_CACHE_DURATION_SECONDS)
                } else {
                    debug!("request for vnstat png file path");

                    match resolved_file.modified {
                        None => Some(0),
                        Some(modified) => {
                            let now = SystemTime::now();

                            let file_expiration = modified + VNSTAT_PNG_CACHE_DURATION;

                            let cache_duration =
                                file_expiration.duration_since(now).unwrap_or_default();

                            debug!(
                                "file_expiration = {:?} cache_duration = {:?}",
                                file_expiration, cache_duration
                            );

                            Some(cache_duration.as_secs().try_into().unwrap_or_default())
                        }
                    }
                }
            }
            _ => None,
        }
    }

    async fn try_handle(&self, request: &HttpRequest) -> anyhow::Result<Response<ResponseBody>> {
        debug!("StaticFileHandler::try_handle request = {:?}", request);

        let resolve_result = self
            .resolver
            .resolve_request(request.hyper_request())
            .await
            .context("try_handle: resolve_request error")?;

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
            .context("try_handle: resolve_request error")?;

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

pub async fn create_default_route() -> Box<dyn RequestHandler> {
    Box::new(StaticFileHandler::new())
}
