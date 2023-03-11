use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::SystemTime,
};

use getset::Getters;

use tokio::sync::{OnceCell, RwLock};

use tracing::debug;

use crate::config::ServerProtocol;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionID(pub usize);

#[derive(Clone, Debug, Getters)]
#[getset(get = "pub")]
pub struct ConnectionInfo {
    id: ConnectionID,
    creation_time: SystemTime,
    server_protocol: ServerProtocol,
    num_requests: Arc<AtomicUsize>,
}

impl ConnectionInfo {
    fn new(id: ConnectionID, server_protocol: ServerProtocol) -> Self {
        Self {
            id,
            creation_time: SystemTime::now(),
            server_protocol,
            num_requests: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn load_num_requests(&self) -> usize {
        self.num_requests.load(Ordering::Relaxed)
    }
}

pub struct ConnectionGuard {
    connection_tracker: &'static ConnectionTracker,
    id: ConnectionID,
    num_requests: Arc<AtomicUsize>,
}

impl ConnectionGuard {
    fn new(
        connection_tracker: &'static ConnectionTracker,
        id: ConnectionID,
        num_requests: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            connection_tracker,
            id,
            num_requests,
        }
    }

    pub fn id(&self) -> ConnectionID {
        self.id
    }

    pub fn increment_num_requests(&self) {
        self.num_requests.fetch_add(1, Ordering::Relaxed);
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let connection_tracker = self.connection_tracker;
        let id = self.id;

        tokio::task::spawn(async move {
            connection_tracker.remove_connection(id).await;
        });
    }
}

struct ConnectionTrackerState {
    next_connection_id: usize,
    id_to_connection_info: HashMap<ConnectionID, ConnectionInfo>,
}

impl ConnectionTrackerState {
    fn new() -> Self {
        Self {
            next_connection_id: 1,
            id_to_connection_info: HashMap::new(),
        }
    }

    fn next_connection_id(&mut self) -> ConnectionID {
        let connection_id = self.next_connection_id;
        self.next_connection_id += 1;
        ConnectionID(connection_id)
    }
}

pub struct ConnectionTracker {
    state: RwLock<ConnectionTrackerState>,
}

impl ConnectionTracker {
    fn new() -> Self {
        Self {
            state: RwLock::new(ConnectionTrackerState::new()),
        }
    }

    pub async fn add_connection(&'static self, server_protocol: ServerProtocol) -> ConnectionGuard {
        let mut state = self.state.write().await;

        let connection_id = state.next_connection_id();

        let connection_info = ConnectionInfo::new(connection_id, server_protocol);

        let num_requests = Arc::clone(connection_info.num_requests());

        state
            .id_to_connection_info
            .insert(connection_id, connection_info);

        debug!(
            "add_new_connection id_to_connection_info.len = {}",
            state.id_to_connection_info.len()
        );

        ConnectionGuard::new(self, connection_id, num_requests)
    }

    async fn remove_connection(&self, connection_id: ConnectionID) {
        let mut state = self.state.write().await;

        state.id_to_connection_info.remove(&connection_id);

        debug!(
            "remove_connection id_to_connection_info.len = {}",
            state.id_to_connection_info.len()
        );
    }

    pub async fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        let state = self.state.read().await;

        state.id_to_connection_info.values().cloned().collect()
    }

    pub async fn instance() -> &'static ConnectionTracker {
        static INSTANCE: OnceCell<ConnectionTracker> = OnceCell::const_new();

        INSTANCE
            .get_or_init(|| async move { ConnectionTracker::new() })
            .await
    }
}
