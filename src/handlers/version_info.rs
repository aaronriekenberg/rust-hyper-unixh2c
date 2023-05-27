use async_trait::async_trait;

use bytes::Bytes;

use http_body_util::combinators::BoxBody;

use hyper::http::{Method, Response, StatusCode};

use std::{convert::Infallible, path::PathBuf};

use crate::{
    handlers::{route::RouteInfo, utils::build_json_response, HttpRequest, RequestHandler},
    version::get_verison_info,
};

struct VersionInfoHandler;

#[async_trait]
impl RequestHandler for VersionInfoHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<BoxBody<Bytes, Infallible>> {
        let version_info = get_verison_info().await;

        build_json_response(version_info)
    }
}

pub async fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("version_info"),
        handler: Box::new(VersionInfoHandler),
    }]
}
