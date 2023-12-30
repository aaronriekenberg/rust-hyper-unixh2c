use async_trait::async_trait;

use hyper::http::{Method, Response, Version};

use serde::Serialize;

use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    handlers::{route::RouteInfo, HttpRequest, RequestHandler},
    response::{build_json_response, CacheControl, ResponseBody},
};

#[derive(Debug, Serialize)]
struct RequestFields<'a> {
    connection_id: usize,
    http_version: &'a str,
    method: &'a str,
    request_id: usize,
    request_uri_path: &'a str,
}

impl<'a> From<&'a HttpRequest> for RequestFields<'a> {
    fn from(request: &'a HttpRequest) -> Self {
        let hyper_request = &request.hyper_request;

        let http_version = match hyper_request.version() {
            Version::HTTP_09 => "HTTP/0.9",
            Version::HTTP_10 => "HTTP/1.0",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2.0",
            Version::HTTP_3 => "HTTP/3.0",
            _ => "[Unknown]",
        };

        Self {
            connection_id: request.connection_id.as_usize(),
            http_version,
            method: hyper_request.method().as_str(),
            request_id: request.request_id.as_usize(),
            request_uri_path: hyper_request.uri().path(),
        }
    }
}

type SortedRequestHeaders<'a> = BTreeMap<&'a str, &'a str>;

impl<'a> From<&'a HttpRequest> for SortedRequestHeaders<'a> {
    fn from(request: &'a HttpRequest) -> Self {
        request
            .hyper_request
            .headers()
            .iter()
            .map(|(key, value)| (key.as_str(), value.to_str().unwrap_or("[Unknown]")))
            .collect()
    }
}

#[derive(Debug, Serialize)]
struct RequestInfoResponse<'a> {
    request_fields: RequestFields<'a>,
    request_headers: SortedRequestHeaders<'a>,
}

impl<'a> From<&'a HttpRequest> for RequestInfoResponse<'a> {
    fn from(request: &'a HttpRequest) -> Self {
        Self {
            request_fields: request.into(),
            request_headers: request.into(),
        }
    }
}

struct RequestInfoHandler;

#[async_trait]
impl RequestHandler for RequestInfoHandler {
    async fn handle(&self, request: &HttpRequest) -> Response<ResponseBody> {
        let response: RequestInfoResponse<'_> = request.into();

        build_json_response(response, CacheControl::NoCache)
    }
}

pub fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("request_info"),
        handler: Box::new(RequestInfoHandler),
    }]
}
