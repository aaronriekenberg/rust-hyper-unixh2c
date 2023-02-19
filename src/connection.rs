use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

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

pub struct ConnectionTracker {
    next_connection_id: AtomicU64,
    id_to_connection_info: RwLock<HashMap<ConnectionID, ConnectionInfo>>,
}

impl ConnectionTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            next_connection_id: AtomicU64::new(1),
            id_to_connection_info: RwLock::new(HashMap::new()),
        })
    }

    fn new_connection_id(&self) -> ConnectionID {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);

        ConnectionID(id)
    }

    pub async fn add_connection(self: &Arc<Self>, server_protocol: ServerProtocol) -> Connection {
        let connection_id = self.new_connection_id();

        let connection_info = ConnectionInfo::new(connection_id, server_protocol);

        let mut id_to_connection_info = self.id_to_connection_info.write().await;

        id_to_connection_info.insert(connection_id, connection_info);

        debug!(
            "add_new_connection id_to_connection_info.len = {}",
            id_to_connection_info.len()
        );

        drop(id_to_connection_info);

        Connection::new(Arc::clone(self), connection_id)
    }

    async fn remove_connection(&self, connection_id: ConnectionID) {
        let mut id_to_connection_info = self.id_to_connection_info.write().await;

        id_to_connection_info.remove(&connection_id);

        debug!(
            "remove_connection id_to_connection_info.len = {}",
            id_to_connection_info.len()
        );
    }

    pub async fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        let id_to_connection_info = self.id_to_connection_info.read().await;

        id_to_connection_info.values().cloned().collect()
    }
}
