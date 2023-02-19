use std::{collections::HashMap, sync::Arc};

use chrono::prelude::{DateTime, Local};

use getset::Getters;

use tokio::sync::RwLock;

use tracing::debug;

use crate::config::ServerProtocol;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionID(pub u64);

#[derive(Clone, Debug, Getters)]
#[getset(get = "pub")]
pub struct ConnectionInfo {
    connection_id: ConnectionID,
    creation_time: DateTime<Local>,
    server_protocol: ServerProtocol,
}

impl ConnectionInfo {
    fn new(connection_id: ConnectionID, server_protocol: ServerProtocol) -> Self {
        Self {
            connection_id,
            creation_time: Local::now(),
            server_protocol,
        }
    }
}

pub struct Connection {
    connection_tracker: Arc<ConnectionTracker>,
    connection_id: ConnectionID,
}

impl Connection {
    fn new(connection_tracker: Arc<ConnectionTracker>, connection_id: ConnectionID) -> Self {
        Self {
            connection_tracker,
            connection_id,
        }
    }

    pub fn connection_id(&self) -> ConnectionID {
        self.connection_id
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let connection_tracker = Arc::clone(&self.connection_tracker);
        let connection_id = self.connection_id;

        tokio::task::spawn(async move {
            connection_tracker.remove_connection(connection_id).await;
        });
    }
}

struct ConnectionTrackerState {
    next_connection_id: u64,
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
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: RwLock::new(ConnectionTrackerState::new()),
        })
    }

    pub async fn add_connection(self: &Arc<Self>, server_protocol: ServerProtocol) -> Connection {
        let mut state = self.state.write().await;

        let connection_id = state.next_connection_id();

        let connection_info = ConnectionInfo::new(connection_id, server_protocol);

        state
            .id_to_connection_info
            .insert(connection_id, connection_info);

        debug!(
            "add_new_connection id_to_connection_info.len = {}",
            state.id_to_connection_info.len()
        );

        drop(state);

        Connection::new(Arc::clone(self), connection_id)
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
}
