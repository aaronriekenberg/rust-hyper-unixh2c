use ahash::AHashMap;

use tokio::time::{Duration, Instant};

use tracing::{debug, warn};

use std::{cmp, sync::Arc};

use crate::config::ServerSocketType;

use super::{ConnectionGuard, ConnectionID, ConnectionInfo};

#[derive(Default)]
struct ConnectionTrackerMetrics {
    max_open_connections: usize,
    connection_limit_hits: usize,
    past_min_connection_age: Option<Duration>,
    past_max_connection_age: Duration,
    past_max_requests_per_connection: usize,
}

impl ConnectionTrackerMetrics {
    fn update_for_new_connection(&mut self, new_num_connections: usize) {
        self.max_open_connections = cmp::max(self.max_open_connections, new_num_connections);
    }

    fn update_for_removed_connection(&mut self, removed_connection_info: &ConnectionInfo) {
        let removed_connection_age = removed_connection_info.age(Instant::now());

        self.past_min_connection_age = Some(cmp::min(
            self.past_min_connection_age.unwrap_or(Duration::MAX),
            removed_connection_age,
        ));

        self.past_max_connection_age =
            cmp::max(self.past_max_connection_age, removed_connection_age);

        self.past_max_requests_per_connection = cmp::max(
            self.past_max_requests_per_connection,
            removed_connection_info.num_requests(),
        );
    }

    fn increment_connection_limit_hits(&mut self) {
        self.connection_limit_hits += 1;
    }
}

#[derive(Default)]
pub struct ConnectionTrackerState {
    next_connection_id: usize,
    connection_limit: usize,
    id_to_connection_info: AHashMap<ConnectionID, Arc<ConnectionInfo>>,
    metrics: ConnectionTrackerMetrics,
}

impl ConnectionTrackerState {
    pub fn new() -> Self {
        let connection_limit = crate::config::instance()
            .server_configuration
            .connection
            .limit;
        Self {
            next_connection_id: 1,
            connection_limit,
            id_to_connection_info: AHashMap::with_capacity(connection_limit),
            ..Default::default()
        }
    }

    fn next_connection_id(&mut self) -> ConnectionID {
        let connection_id = self.next_connection_id;
        self.next_connection_id += 1;
        ConnectionID(connection_id)
    }

    fn new_connection_exceeds_connection_limit(&self) -> bool {
        (self.id_to_connection_info.len() + 1) > self.connection_limit
    }

    pub fn add_connection(
        &mut self,
        server_socket_type: ServerSocketType,
    ) -> Option<ConnectionGuard> {
        if self.new_connection_exceeds_connection_limit() {
            warn!(
                "add_connection hit connection_limit = {} server_socket_type = {:?}",
                self.connection_limit, server_socket_type
            );
            self.metrics.increment_connection_limit_hits();
            return None;
        }

        let connection_id = self.next_connection_id();

        let connection_info = Arc::new(ConnectionInfo::new(connection_id, server_socket_type));

        let num_requests = Arc::clone(&connection_info.num_requests);

        self.id_to_connection_info
            .insert(connection_id, connection_info);

        let new_num_connections = self.id_to_connection_info.len();

        self.metrics.update_for_new_connection(new_num_connections);

        debug!(
            "add_connection new_num_connections = {}",
            new_num_connections
        );

        Some(ConnectionGuard::new(
            connection_id,
            server_socket_type,
            num_requests,
        ))
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

    pub fn connection_limit_hits(&self) -> usize {
        self.metrics.connection_limit_hits
    }

    pub fn min_connection_lifetime(&self) -> Duration {
        match self.metrics.past_min_connection_age {
            Some(past_min_connection_age) => past_min_connection_age,
            None => {
                let now = Instant::now();
                self.id_to_connection_info
                    .values()
                    .map(|c| c.age(now))
                    .min()
                    .unwrap_or_default()
            }
        }
    }

    pub fn max_connection_lifetime(&self) -> Duration {
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

    pub fn open_connections(&self) -> impl Iterator<Item = &Arc<ConnectionInfo>> {
        self.id_to_connection_info.values()
    }
}
