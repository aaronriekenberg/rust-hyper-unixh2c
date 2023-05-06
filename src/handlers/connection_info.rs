use std::path::PathBuf;

use async_trait::async_trait;

use hyper::{Body, Method, Response};

use serde::Serialize;

use std::time::Duration;

use crate::{
    config::ServerProtocol,
    connection::{ConnectionInfo, ConnectionTracker, ConnectionTrackerState},
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

        Self {
            id: connection_info.id().0,
            server_protocol: *connection_info.server_protocol(),
            creation_time: local_date_time_to_string(&LocalDateTime::from(
                *connection_info.creation_time(),
            )),
            age,
            num_requests: connection_info.num_requests(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ConnectionTrackerStateDTO {
    max_open_connections: usize,
    #[serde(with = "humantime_serde")]
    max_connection_lifetime: Duration,
    max_requests_per_connection: usize,
    num_open_connections: usize,
    open_connections: Vec<ConnectionInfoDTO>,
}

impl From<ConnectionTrackerState> for ConnectionTrackerStateDTO {
    fn from(state: ConnectionTrackerState) -> Self {
        let mut open_connections: Vec<ConnectionInfoDTO> = state
            .open_connections
            .into_iter()
            .map(|c| c.into())
            .collect();

        open_connections.sort_unstable_by_key(|c| c.id);

        let open_connections = open_connections;

        let num_open_connections = open_connections.len();

        let open_connections = open_connections.into_iter().take(20).collect();

        // truncate to seconds
        let max_connection_lifetime = Duration::from_secs(state.max_connection_lifetime.as_secs());

        Self {
            max_open_connections: state.max_open_connections,
            max_connection_lifetime,
            max_requests_per_connection: state.max_requests_per_connection,
            num_open_connections,
            open_connections,
        }
    }
}

struct ServerInfoHandler {
    connection_tracker: &'static ConnectionTracker,
}

impl ServerInfoHandler {
    async fn new() -> Self {
        Self {
            connection_tracker: ConnectionTracker::instance().await,
        }
    }
}

#[async_trait]
impl RequestHandler for ServerInfoHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<Body> {
        let connection_tracker_state_dto: ConnectionTrackerStateDTO =
            self.connection_tracker.state().await.into();

        build_json_response(connection_tracker_state_dto)
    }
}

pub async fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("connection_info"),
        handler: Box::new(ServerInfoHandler::new().await),
    }]
}
