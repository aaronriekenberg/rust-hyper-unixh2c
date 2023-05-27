use bytes::Bytes;

use tracing::warn;

use serde::Serialize;

use hyper::http::{header, Response, StatusCode};

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};

use std::convert::Infallible;

pub fn build_json_body_response(
    http_response_body: BoxBody<Bytes, Infallible>,
) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(http_response_body)
        .unwrap()
}

pub fn build_json_response(response_dto: impl Serialize) -> Response<BoxBody<Bytes, Infallible>> {
    let json_result = serde_json::to_string(&response_dto);

    match json_result {
        Err(e) => {
            warn!("build_json_response serialization error {}", e);

            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Empty::new().boxed())
                .unwrap()
        }
        Ok(json_string) => {
            build_json_body_response(Full::new(json_string.into()).boxed())
        }
    }
}

pub fn build_status_code_response(status_code: StatusCode) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(status_code)
        .body(Empty::new().boxed())
        .unwrap()
}
