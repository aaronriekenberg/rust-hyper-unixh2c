use std::path::PathBuf;

use async_trait::async_trait;

use hyper::{Body, Method, Response};

use crate::{
    handlers::{route::RouteInfo, utils::build_json_response, HttpRequest, RequestHandler},
    version::get_verison_info,
};

struct VersionInfoHandler;

#[async_trait]
impl RequestHandler for VersionInfoHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<Body> {
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
