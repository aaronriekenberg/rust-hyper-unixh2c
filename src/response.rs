use bytes::Bytes;

use http_body_util::{
    combinators::BoxBody,
    {BodyExt, Empty, Full},
};

use hyper::http::{header, HeaderValue, Response, StatusCode};

use serde::Serialize;

use tracing::warn;

use std::convert::Infallible;

#[derive(Clone, Copy, Debug)]
pub enum CacheControl {
    NoCache,
    // Cache { max_age_seconds: u32 },
}

impl CacheControl {
    pub fn header_value(&self) -> HeaderValue {
        static NO_CACHE_VALUE: HeaderValue = HeaderValue::from_static("public, no-cache");

        match self {
            CacheControl::NoCache => NO_CACHE_VALUE.clone(),
            // CacheControl::Cache { max_age_seconds } => {
            //    format!("public, max-age={}", max_age_seconds)
            // }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ResponseBodyError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<Infallible> for ResponseBodyError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub type ResponseBody = BoxBody<Bytes, ResponseBodyError>;

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
