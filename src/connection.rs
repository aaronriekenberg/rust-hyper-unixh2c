use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};


use chrono::prelude::{DateTime, Local};

use getset::Getters;


use tokio::sync::OnceCell;

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

pub struct ConnectionGuard {
    connection_tracker: &'static ConnectionTracker,
    connection_id: ConnectionID,
}

impl ConnectionGuard {
    fn new(connection_tracker: &'static ConnectionTracker, connection_id: ConnectionID) -> Self {
        Self {
            connection_tracker,
            connection_id,
        }
    }

    pub fn connection_id(&self) -> ConnectionID {
        self.connection_id
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.connection_tracker
            .remove_connection(self.connection_id)
    }
}

pub struct ConnectionTracker {
    next_connection_id: AtomicU64,
    id_to_connection_info: Mutex<HashMap<ConnectionID, ConnectionInfo>>,
}

impl ConnectionTracker {
    fn new() -> Self {
        Self {
            next_connection_id: AtomicU64::new(1),
            id_to_connection_info: Mutex::new(HashMap::new()),
        }
    }

    fn new_connection_id(&self) -> ConnectionID {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);

        ConnectionID(id)
    }

    pub fn add_connection(&'static self, server_protocol: ServerProtocol) -> ConnectionGuard {
        let connection_id = self.new_connection_id();

        let connection_info = ConnectionInfo::new(connection_id, server_protocol);

        let mut id_to_connection_info = self.id_to_connection_info.lock().unwrap();

        id_to_connection_info.insert(connection_id, connection_info);

        debug!(
            "add_new_connection id_to_connection_info.len = {}",
            id_to_connection_info.len()
        );

        ConnectionGuard::new(&self, connection_id)
    }

    fn remove_connection(&self, connection_id: ConnectionID) {
        let mut id_to_connection_info = self.id_to_connection_info.lock().unwrap();

        id_to_connection_info.remove(&connection_id);

        debug!(
            "remove_connection id_to_connection_info.len = {}",
            id_to_connection_info.len()
        );
    }

    pub fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        let id_to_connection_info = self.id_to_connection_info.lock().unwrap();

        id_to_connection_info.values().cloned().collect()
    }
}

static CONNECTION_TRACKER: OnceCell<ConnectionTracker> = OnceCell::const_new();

pub async fn get_connection_tracker() -> &'static ConnectionTracker {
    CONNECTION_TRACKER
        .get_or_init(|| async { ConnectionTracker::new() })
        .await
}
