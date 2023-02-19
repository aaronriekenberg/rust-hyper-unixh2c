use std::{convert::From, path::PathBuf, sync::Arc};

use async_trait::async_trait;

use hyper::{Body, Method, Response};

use serde::Serialize;

use std::time::Duration;

use crate::{
    config::ServerProtocol,
    connection::{ConnectionInfo, ConnectionTracker},
    handlers::{
        route::RouteInfo,
        utils::{build_json_response, current_local_date_time, local_date_time_to_string},
        HttpRequest, RequestHandler,
    },
};

#[derive(Debug, Serialize)]
struct ConnectionInfoDTO {
    connection_id: u64,
    server_protocol: ServerProtocol,
    creation_time: String,
    #[serde(with = "humantime_serde")]
    age: Duration,
}

impl From<&ConnectionInfo> for ConnectionInfoDTO {
    fn from(connection_info: &ConnectionInfo) -> Self {
        let now = current_local_date_time();

        let age = (now - *connection_info.creation_time())
            .to_std()
            .unwrap_or_default();

        ConnectionInfoDTO {
            connection_id: connection_info.connection_id().0,
            server_protocol: *connection_info.server_protocol(),
            creation_time: local_date_time_to_string(connection_info.creation_time()),
            age,
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
