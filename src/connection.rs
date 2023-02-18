use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use tokio::time::Instant;

use tracing::info;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionID(pub u64);

pub struct ConnectionInfo {
    creation_time: Instant,
}

impl ConnectionInfo {
    fn new() -> Self {
        Self {
            creation_time: Instant::now(),
        }
    }
}

pub struct ConnectionGuard {
    connection_tracker: Arc<ConnectionTracker>,
    connection_id: ConnectionID,
}

impl ConnectionGuard {
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

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        Arc::clone(&self.connection_tracker).remove_connection(self.connection_id)
    }
}

pub struct ConnectionTracker {
    next_connection_id: AtomicU64,
    id_to_connection_info: Mutex<HashMap<ConnectionID, ConnectionInfo>>,
}

impl ConnectionTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            next_connection_id: AtomicU64::new(1),
            id_to_connection_info: Mutex::new(HashMap::new()),
        })
    }

    fn new_connection_id(&self) -> ConnectionID {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);

        ConnectionID(id)
    }

    pub fn add_new_connection(self: &Arc<Self>) -> ConnectionGuard {
        let connection_id = self.new_connection_id();

        let mut id_to_connection_info = self.id_to_connection_info.lock().unwrap();

        id_to_connection_info.insert(connection_id, ConnectionInfo::new());

        info!(
            "add_new_connection id_to_connection_info.len = {}",
            id_to_connection_info.len()
        );

        ConnectionGuard::new(Arc::clone(self), connection_id)
    }

    fn remove_connection(self: Arc<Self>, connection_id: ConnectionID) {
        let mut id_to_connection_info = self.id_to_connection_info.lock().unwrap();

        id_to_connection_info.remove(&connection_id);

        info!(
            "remove_connection id_to_connection_info.len = {}",
            id_to_connection_info.len()
        );
    }
}
