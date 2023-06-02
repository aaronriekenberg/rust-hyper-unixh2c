use tokio::time::{Duration, Instant};

use tracing::debug;

use std::{cmp, collections::HashMap, sync::Arc};

use crate::config::{ServerProtocol, ServerSocketType};

use super::{ConnectionGuard, ConnectionID, ConnectionInfo};

#[derive(Default)]
struct ConnectionTrackerMetrics {
    max_open_connections: usize,
    past_max_connection_age: Duration,
    past_max_requests_per_connection: usize,
}

impl ConnectionTrackerMetrics {
    fn update_for_new_connection(&mut self, num_connections: usize) {
        self.max_open_connections = cmp::max(self.max_open_connections, num_connections);
    }

    fn update_for_removed_connection(&mut self, removed_connection_info: &ConnectionInfo) {
        self.past_max_connection_age = cmp::max(
            self.past_max_connection_age,
            removed_connection_info.age(Instant::now()),
        );

        self.past_max_requests_per_connection = cmp::max(
            self.past_max_requests_per_connection,
            removed_connection_info.num_requests(),
        );
    }
}

#[derive(Default)]
pub struct ConnectionTrackerState {
    next_connection_id: usize,
    id_to_connection_info: HashMap<ConnectionID, ConnectionInfo>,
    metrics: ConnectionTrackerMetrics,
}

impl ConnectionTrackerState {
    pub fn new() -> Self {
        Self {
            next_connection_id: 1,
            ..Default::default()
        }
    }

    fn next_connection_id(&mut self) -> ConnectionID {
        let connection_id = self.next_connection_id;
        self.next_connection_id += 1;
        ConnectionID(connection_id)
    }

    pub fn add_connection(
        &mut self,
        server_protocol: ServerProtocol,
        server_socket_type: ServerSocketType,
    ) -> ConnectionGuard {
        let connection_id = self.next_connection_id();

        let connection_info =
            ConnectionInfo::new(connection_id, server_protocol, server_socket_type);

        let num_requests = Arc::clone(&connection_info.num_requests);

        self.id_to_connection_info
            .insert(connection_id, connection_info);

        let num_connections = self.id_to_connection_info.len();

        self.metrics.update_for_new_connection(num_connections);

        debug!("add_connection num_connections = {}", num_connections);

        ConnectionGuard::new(connection_id, num_requests)
    }

    pub fn remove_connection(&mut self, connection_id: ConnectionID) {
        if let Some(connection_info) = self.id_to_connection_info.remove(&connection_id) {
            self.metrics.update_for_removed_connection(&connection_info);
        }

        debug!(
            "remove_connection id_to_connection_info.len = {}",
            self.id_to_connection_info.len()
        );
    }

    pub fn max_open_connections(&self) -> usize {
        self.metrics.max_open_connections
    }

    pub fn max_connection_age(&self) -> Duration {
        let now = Instant::now();
        cmp::max(
            self.metrics.past_max_connection_age,
            self.id_to_connection_info
                .values()
                .map(|c| c.age(now))
                .max()
                .unwrap_or_default(),
        )
    }

    pub fn max_requests_per_connection(&self) -> usize {
        cmp::max(
            self.metrics.past_max_requests_per_connection,
            self.id_to_connection_info
                .values()
                .map(|c| c.num_requests())
                .max()
                .unwrap_or_default(),
        )
    }

    pub fn open_connections(&self) -> impl Iterator<Item = &ConnectionInfo> {
        self.id_to_connection_info.values()
    }
}
