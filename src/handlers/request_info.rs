use std::{collections::BTreeMap, path::PathBuf};

use async_trait::async_trait;

use hyper::{http::Method, http::Version, Body, Response};

use serde::Serialize;

use crate::handlers::{route::RouteInfo, utils::build_json_response, HttpRequest, RequestHandler};

#[derive(Debug, Serialize)]
struct RequestFields<'a> {
    connection_id: usize,
    http_version: &'a str,
    method: &'a str,
    request_id: usize,
    request_uri_path: &'a str,
}

#[derive(Debug, Serialize)]
struct RequestInfoResponse<'a> {
    app_version: &'a str,
    request_fields: RequestFields<'a>,
    request_headers: BTreeMap<&'a str, &'a str>,
}

struct RequestInfoHandler {
    app_version: &'static str,
}

impl RequestInfoHandler {
    fn new() -> Self {
        let app_version = env!("VERGEN_BUILD_SEMVER");
        Self { app_version }
    }
}

#[async_trait]
impl RequestHandler for RequestInfoHandler {
    async fn handle(&self, request: &HttpRequest) -> Response<Body> {
        let hyper_request = request.hyper_request();

        let http_version = match hyper_request.version() {
            Version::HTTP_09 => "HTTP/0.9",
            Version::HTTP_10 => "HTTP/1.0",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2.0",
            Version::HTTP_3 => "HTTP/3.0",
            _ => "[Unknown]",
        };

        let response = RequestInfoResponse {
            app_version: self.app_version,
            request_fields: RequestFields {
                connection_id: request.connection_id().0,
                http_version,
                method: hyper_request.method().as_str(),
                request_id: request.request_id().0,
                request_uri_path: hyper_request.uri().path(),
            },
            request_headers: hyper_request
                .headers()
                .iter()
                .map(|(key, value)| (key.as_str(), value.to_str().unwrap_or("[Unknown]")))
                .collect(),
        };

        build_json_response(response)
    }
}

pub fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("request_info"),
        handler: Box::new(RequestInfoHandler::new()),
    }]
}
