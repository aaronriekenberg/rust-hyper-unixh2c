use async_trait::async_trait;

use http_body_util::BodyExt;

use hyper::http::{Response, StatusCode};

use hyper_staticfile::{vfs::TokioFileOpener, ResolveResult, Resolver};

use tracing::{debug, warn};

use std::{path::Path, time::SystemTime};

use tokio::time::Duration;

use crate::{
    handlers::{utils::build_status_code_response, HttpRequest, RequestHandler, ResponseBody},
    response::CacheControl,
};

const ONE_DAY_IN_SECONDS: u32 = 24 * 3600;
const VNSTAT_PNG_CACHE_DURATION: Duration = Duration::from_secs(15 * 60);

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

    fn block_dot_paths(&self, resolve_result: &ResolveResult) -> Option<Response<ResponseBody>> {
        let str_path_option = match resolve_result {
            ResolveResult::Found(resolved_file) => resolved_file.path.to_str(),
            ResolveResult::IsDirectory { redirect_to } => Some(redirect_to.as_str()),
            _ => None,
        };

        if let Some(str_path) = str_path_option {
            debug!("str_path = {}", str_path);
            if str_path.starts_with('.') || str_path.contains("/.") {
                warn!("blocking request for dot file path = {:?}", str_path);
                return Some(build_status_code_response(
                    StatusCode::FORBIDDEN,
                    CacheControl::NoCache,
                ));
            }
        };

        None
    }

    fn build_cache_headers(&self, resolve_result: &ResolveResult) -> Option<u32> {
        match resolve_result {
            ResolveResult::Found(resolved_file) => {
                debug!("resolved_file.path = {:?}", resolved_file.path,);

                let str_path = resolved_file.path.to_str().unwrap_or_default();

                if str_path.contains("vnstat/") && str_path.ends_with(".png") {
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
                } else {
                    Some(ONE_DAY_IN_SECONDS)
                }
            }
            _ => None,
        }
    }
}

#[async_trait]
impl RequestHandler for StaticFileHandler {
    async fn handle(&self, request: &HttpRequest) -> Response<ResponseBody> {
        debug!("handle_static_file request = {:?}", request);

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

        debug!("resolve_result = {:?}", resolve_result);

        if let Some(response) = self.block_dot_paths(&resolve_result) {
            return response;
        }

        let cache_headers = self.build_cache_headers(&resolve_result);

        debug!("cache_headers = {:?}", cache_headers);

        let response = match hyper_staticfile::ResponseBuilder::new()
            .request(request.hyper_request())
            .cache_headers(cache_headers)
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
