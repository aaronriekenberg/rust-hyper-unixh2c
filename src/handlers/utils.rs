use tracing::warn;

use serde::Serialize;

use hyper::http::{header, Response, StatusCode};

use http_body_util::{BodyExt, Empty, Full};

use super::ResponseBody;

pub fn build_json_body_response(http_response_body: ResponseBody) -> Response<ResponseBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(http_response_body)
        .unwrap()
}

pub fn build_json_response(response_dto: impl Serialize) -> Response<ResponseBody> {
    let json_result = serde_json::to_string(&response_dto);

    match json_result {
        Err(e) => {
            warn!("build_json_response serialization error {}", e);

            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(empty_response_body())
                .unwrap()
        }
        Ok(json_string) => build_json_body_response(
            Full::new(json_string.into())
                .map_err(|never| never.into())
                .boxed(),
        ),
    }
}

pub fn build_status_code_response(status_code: StatusCode) -> Response<ResponseBody> {
    Response::builder()
        .status(status_code)
        .body(empty_response_body())
        .unwrap()
}

pub fn empty_response_body() -> ResponseBody {
    Empty::new().map_err(|never| never.into()).boxed()
}
