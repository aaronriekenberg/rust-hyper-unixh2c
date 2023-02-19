use chrono::prelude::{DateTime, Local, SecondsFormat};

use tracing::warn;

use serde::Serialize;

use hyper::{header, http::StatusCode, Body, Response};

pub fn build_json_body_response(http_response_body: Body) -> Response<hyper::Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(http_response_body)
        .unwrap()
}

pub fn build_json_response(response_dto: impl Serialize) -> Response<Body> {
    let json_result = serde_json::to_string(&response_dto);

    match json_result {
        Err(e) => {
            warn!("build_json_response serialization error {}", e);

            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        }
        Ok(json_string) => build_json_body_response(Body::from(json_string)),
    }
}

pub fn build_status_code_response(status_code: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status_code)
        .body(Body::empty())
        .unwrap()
}

pub fn current_local_date_time() -> DateTime<Local> {
    Local::now()
}

pub fn local_date_time_to_string(local_date_time: &DateTime<Local>) -> String {
    local_date_time.to_rfc3339_opts(SecondsFormat::Millis, false)
}

pub fn current_local_date_time_string() -> String {
    local_date_time_to_string(&current_local_date_time())
}
