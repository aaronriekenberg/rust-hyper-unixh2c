use getset::Getters;

use hyper::{http::Request, Body};

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::connection::ConnectionID;

#[derive(Clone, Copy, Debug)]
pub struct RequestID(pub usize);

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct HttpRequest {
    connection_id: ConnectionID,
    request_id: RequestID,
    hyper_request: Request<Body>,
}

impl HttpRequest {
    pub fn new(
        connection_id: ConnectionID,
        request_id: RequestID,
        hyper_request: Request<Body>,
    ) -> Self {
        Self {
            connection_id,
            request_id,
            hyper_request,
        }
    }
}

pub struct RequestIDFactory {
    next_request_id: AtomicUsize,
}

impl RequestIDFactory {
    pub fn new() -> Self {
        Self {
            next_request_id: AtomicUsize::new(1),
        }
    }

    pub fn new_request_id(&self) -> RequestID {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);

        RequestID(id)
    }
}
