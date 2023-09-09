use std::path::PathBuf;

use async_trait::async_trait;

use hyper::http::{Method, Response};

use serde::Serialize;

use tokio::time::Instant;

use std::{collections::BTreeMap, time::Duration};

use crate::{
    config::{ServerProtocol, ServerSocketType},
    connection::{ConnectionID, ConnectionInfo, ConnectionTracker, ConnectionTrackerState},
    handlers::{
        response_utils::build_json_response,
        route::RouteInfo,
        time_utils::{local_date_time_to_string, LocalDateTime},
        HttpRequest, RequestHandler, ResponseBody,
    },
    response::CacheControl,
};

#[derive(Debug, Serialize)]
struct ConnectionInfoDTO {
    id: usize,
    server_protocol: ServerProtocol,
    server_socket_type: ServerSocketType,
    creation_time: String,
    #[serde(with = "humantime_serde")]
    age: Duration,
    num_requests: usize,
}

impl From<ConnectionInfo> for ConnectionInfoDTO {
    fn from(connection_info: ConnectionInfo) -> Self {
        // truncate to seconds
        let age = Duration::from_secs(connection_info.age(Instant::now()).as_secs());

        Self {
            id: connection_info.id.as_usize(),
            server_protocol: connection_info.server_protocol,
            server_socket_type: connection_info.server_socket_type,
            creation_time: local_date_time_to_string(&LocalDateTime::from(
                connection_info.creation_time,
            )),
            age,
            num_requests: connection_info.num_requests(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ConnectionTrackerStateDTO {
    max_open_connections: usize,
    connection_limit_hits: usize,
    #[serde(with = "humantime_serde")]
    max_connection_lifetime: Duration,
    max_requests_per_connection: usize,
    num_open_connections: usize,
    open_connections: Vec<ConnectionInfoDTO>,
}

impl From<ConnectionTrackerState> for ConnectionTrackerStateDTO {
    fn from(state: ConnectionTrackerState) -> Self {
        let id_to_open_connection: BTreeMap<ConnectionID, ConnectionInfo> = state
            .open_connections
            .into_iter()
            .map(|c| (c.id, c))
            .collect();

        let num_open_connections = id_to_open_connection.len();

        // 20 newest connections with descending ids in reverse order
        let open_connections = id_to_open_connection
            .into_iter()
            .rev()
            .take(20)
            .map(|(_, v)| v.into())
            .collect();

        // truncate to seconds
        let max_connection_lifetime = Duration::from_secs(state.max_connection_age.as_secs());

        Self {
            max_open_connections: state.max_open_connections,
            connection_limit_hits: state.connection_limit_hits,
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
    async fn handle(&self, _request: &HttpRequest) -> Response<ResponseBody> {
        let connection_tracker_state_dto: ConnectionTrackerStateDTO =
            self.connection_tracker.state().await.into();

        build_json_response(connection_tracker_state_dto, CacheControl::NoCache)
    }
}

pub async fn create_routes() -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("connection_info"),
        handler: Box::new(ServerInfoHandler::new().await),
    }]
}
