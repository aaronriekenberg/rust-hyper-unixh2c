use std::{convert::From, path::PathBuf, sync::Arc};

use async_trait::async_trait;

use hyper::{Body, Method, Response};

use serde::Serialize;

use std::time::Duration;

use crate::{
    config::ServerProtocol,
    connection::{ConnectionInfo, ConnectionTracker},
    handlers::{route::RouteInfo, utils::build_json_response, HttpRequest, RequestHandler},
    time::{local_date_time_to_string, LocalDateTime},
};

#[derive(Debug, Serialize)]
struct ConnectionInfoDTO {
    connection_id: u64,
    server_protocol: ServerProtocol,
    creation_time: String,
    #[serde(with = "humantime_serde")]
    age: Duration,
    num_requests: u64,
}

impl From<&ConnectionInfo> for ConnectionInfoDTO {
    fn from(connection_info: &ConnectionInfo) -> Self {
        let age = connection_info
            .creation_time()
            .elapsed()
            .unwrap_or_default();

        // truncate to milliseconds
        let age = (age.as_secs_f64() * 1000.0).trunc() / 1000.0;

        let age = Duration::from_secs_f64(age);

        ConnectionInfoDTO {
            connection_id: connection_info.connection_id().0,
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
    connections: Vec<ConnectionInfoDTO>,
}

struct ConnectionInfoHandler {
    connection_tracker: Arc<ConnectionTracker>,
}

impl ConnectionInfoHandler {
    fn new(connection_tracker: &Arc<ConnectionTracker>) -> Self {
        Self {
            connection_tracker: Arc::clone(connection_tracker),
        }
    }
}

#[async_trait]
impl RequestHandler for ConnectionInfoHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<Body> {
        let mut connections: Vec<ConnectionInfoDTO> = self
            .connection_tracker
            .get_all_connections()
            .await
            .iter()
            .map(|c| c.into())
            .collect();

        connections.sort_by_key(|c| c.connection_id);

        let response = ConnectionInfoResponse { connections };

        build_json_response(response)
    }
}

pub fn create_routes(connection_tracker: &Arc<ConnectionTracker>) -> Vec<RouteInfo> {
    vec![RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("connection_info"),
        handler: Box::new(ConnectionInfoHandler::new(connection_tracker)),
    }]
}
