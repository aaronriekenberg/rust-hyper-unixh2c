use tracing::warn;

use serde::Serialize;

use hyper::http::{header, Response, StatusCode};

use http_body_util::{BodyExt, Empty, Full};

use crate::response::CacheControl;

use super::ResponseBody;

pub fn build_json_body_response(
    http_response_body: ResponseBody,
    cache_control: CacheControl,
) -> Response<ResponseBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, cache_control.header_value())
        .body(http_response_body)
        .unwrap()
}

pub fn build_json_response(
    response_dto: impl Serialize,
    cache_control: CacheControl,
) -> Response<ResponseBody> {
    let json_result = serde_json::to_string(&response_dto);

    match json_result {
        Err(e) => {
            warn!("build_json_response serialization error {}", e);

            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CACHE_CONTROL, CacheControl::NoCache.header_value())
                .body(empty_response_body())
                .unwrap()
        }
        Ok(json_string) => build_json_body_response(
            Full::from(json_string)
                .map_err(|never| never.into())
                .boxed(),
            cache_control,
        ),
    }
}

pub fn build_status_code_response(
    status_code: StatusCode,
    cache_control: CacheControl,
) -> Response<ResponseBody> {
    Response::builder()
        .status(status_code)
        .header(header::CACHE_CONTROL, cache_control.header_value())
        .body(empty_response_body())
        .unwrap()
}

pub fn empty_response_body() -> ResponseBody {
    Empty::new().map_err(|never| never.into()).boxed()
}

pub fn static_string_response_body(s: &'static str) -> ResponseBody {
    Full::from(s).map_err(|e| e.into()).boxed()
}
