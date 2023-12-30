use async_trait::async_trait;

use hyper::http::{Method, Response};

use std::path::PathBuf;

use crate::{
    handlers::{route::RouteInfo, HttpRequest, RequestHandler, ResponseBody},
    response::{build_json_response, CacheControl},
    version::get_verison_info,
};

struct VersionInfoHandler;

#[async_trait]
impl RequestHandler for VersionInfoHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<ResponseBody> {
        let version_info = get_verison_info().await;

        build_json_response(version_info, CacheControl::NoCache)
    }
}

pub async fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("version_info"),
        handler: Box::new(VersionInfoHandler),
    }]
}
