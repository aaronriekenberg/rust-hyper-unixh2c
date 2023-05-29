use getset::{CopyGetters, Getters};

use hyper::{body::Incoming, http::Request};

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::connection::ConnectionID;

#[derive(Clone, Copy, Debug)]
pub struct RequestID(usize);

impl RequestID {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Getters, CopyGetters)]
pub struct HttpRequest {
    #[getset(get_copy = "pub")]
    connection_id: ConnectionID,

    #[getset(get_copy = "pub")]
    request_id: RequestID,

    #[getset(get = "pub")]
    hyper_request: Request<Incoming>,
}

impl HttpRequest {
    pub fn new(
        connection_id: ConnectionID,
        request_id: RequestID,
        hyper_request: Request<Incoming>,
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
