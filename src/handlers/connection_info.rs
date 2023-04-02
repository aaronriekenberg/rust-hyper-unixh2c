use std::{convert::From, path::PathBuf};

use async_trait::async_trait;

use hyper::{Body, Method, Response};

use serde::Serialize;

use std::time::Duration;

use crate::{
    config::ServerProtocol,
    connection::{ConnectionInfo, ConnectionTracker, ConnectionTrackerInfo},
    handlers::{route::RouteInfo, utils::build_json_response, HttpRequest, RequestHandler},
    time::{local_date_time_to_string, LocalDateTime},
};

#[derive(Debug, Serialize)]
struct ConnectionInfoDTO {
    id: usize,
    server_protocol: ServerProtocol,
    creation_time: String,
    #[serde(with = "humantime_serde")]
    age: Duration,
    num_requests: usize,
}

impl From<ConnectionInfo> for ConnectionInfoDTO {
    fn from(connection_info: ConnectionInfo) -> Self {
        let age = connection_info
            .creation_time()
            .elapsed()
            .unwrap_or_default();

        // truncate to seconds
        let age = Duration::from_secs(age.as_secs());

        ConnectionInfoDTO {
            id: connection_info.id().0,
            server_protocol: *connection_info.server_protocol(),
            creation_time: local_date_time_to_string(&LocalDateTime::from(
                *connection_info.creation_time(),
            )),
            age,
            num_requests: connection_info.load_num_requests(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ConnectionInfoResponse {
    max_open_connections: usize,
    num_open_connections: usize,
    open_connections: Vec<ConnectionInfoDTO>,
}

impl From<ConnectionTrackerInfo> for ConnectionInfoResponse {
    fn from(metadata: ConnectionTrackerInfo) -> Self {
        let mut open_connections: Vec<ConnectionInfoDTO> = metadata
            .open_connections
            .into_iter()
            .map(|c| c.into())
            .collect();

        open_connections.sort_by_key(|c| c.id);

        ConnectionInfoResponse {
            max_open_connections: metadata.max_open_connections,
            num_open_connections: open_connections.len(),
            open_connections,
        }
    }
}

struct ConnectionInfoHandler {
    connection_tracker: &'static ConnectionTracker,
}

impl ConnectionInfoHandler {
    async fn new() -> Self {
        Self {
            connection_tracker: ConnectionTracker::instance().await,
        }
    }
}

#[async_trait]
impl RequestHandler for ConnectionInfoHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<Body> {
        let response: ConnectionInfoResponse = self.connection_tracker.get_info().await.into();

        build_json_response(response)
    }
}

pub async fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("connection_info"),
        handler: Box::new(ConnectionInfoHandler::new().await),
    }]
}
