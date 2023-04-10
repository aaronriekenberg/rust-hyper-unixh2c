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
    version::{get_verison_info, VersionInfoMap},
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
struct ConnectionTrackerStateDTO {
    max_open_connections: usize,
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

        Self {
            max_open_connections: state.max_open_connections,
            num_open_connections: open_connections.len(),
            open_connections,
        }
    }
}

#[derive(Debug, Serialize)]
struct ServerInfoDTO {
    connection_info: ConnectionTrackerStateDTO,
    version_info: VersionInfoMap,
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
        let server_info_dto = ServerInfoDTO {
            connection_info: self.connection_tracker.state().await.into(),
            version_info: get_verison_info(),
        };

        build_json_response(server_info_dto)
    }
}

pub async fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("server_info"),
        handler: Box::new(ServerInfoHandler::new().await),
    }]
}
