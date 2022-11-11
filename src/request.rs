use getset::Getters;

use hyper::{http::Request, Body};

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct HttpRequest {
    connection_id: u64,
    request_id: u64,
    hyper_request: Request<Body>,
}

impl HttpRequest {
    pub fn new(connection_id: u64, request_id: u64, hyper_request: Request<Body>) -> Self {
        Self {
            connection_id,
            request_id,
            hyper_request,
        }
    }
}
