use std::{collections::BTreeMap, path::PathBuf};

use async_trait::async_trait;

use hyper::{http::Version, Body, Request, Response};

use serde::Serialize;

use crate::handlers::{route::PathSuffixAndHandler, utils::build_json_response, RequestHandler};

#[derive(Debug, Serialize)]
struct RequestInfoResponse<'a> {
    version: &'a str,
    request_uri: String,
    http_headers: BTreeMap<&'a str, &'a str>,
}

struct RequestInfoHandler {}

impl RequestInfoHandler {
    fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl RequestHandler for RequestInfoHandler {
    async fn handle(&self, request: Request<Body>) -> Response<Body> {
        let version = match request.version() {
            Version::HTTP_09 => "HTTP/0.9",
            Version::HTTP_10 => "HTTP/1.0",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2.0",
            Version::HTTP_3 => "HTTP/3.0",
            _ => "[Unknown]",
        };
        
        let mut response = RequestInfoResponse {
            version,
            request_uri: request.uri().to_string(),
            http_headers: BTreeMap::new(),
        };

        for (key, value) in request.headers().iter() {
            response
                .http_headers
                .insert(key.as_str(), value.to_str().unwrap_or("[Unknown]"));
        }

        build_json_response(response)
    }
}

pub fn create_routes() -> Vec<PathSuffixAndHandler> {
    vec![(
        PathBuf::from("request_info"),
        Box::new(RequestInfoHandler::new()),
    )]
}
