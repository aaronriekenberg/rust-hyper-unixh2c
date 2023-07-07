mod internal;

use getset::CopyGetters;

use tokio::{
    sync::{OnceCell, RwLock},
    time::{Duration, Instant},
};

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::SystemTime,
};

use crate::config::{ServerProtocol, ServerSocketType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ConnectionID(usize);

impl ConnectionID {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct ConnectionInfo {
    id: ConnectionID,
    creation_time: SystemTime,
    creation_instant: Instant,
    server_protocol: ServerProtocol,
    server_socket_type: ServerSocketType,
    #[getset(skip)]
    num_requests: Arc<AtomicUsize>,
}

impl ConnectionInfo {
    fn new(
        id: ConnectionID,
        server_protocol: ServerProtocol,
        server_socket_type: ServerSocketType,
    ) -> Self {
        Self {
            id,
            creation_time: SystemTime::now(),
            creation_instant: Instant::now(),
            server_protocol,
            server_socket_type,
            num_requests: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn num_requests(&self) -> usize {
        self.num_requests.load(Ordering::Relaxed)
    }

    pub fn age(&self, now: Instant) -> Duration {
        now - self.creation_instant
    }
}

#[derive(CopyGetters)]
#[getset(get_copy = "pub")]
pub struct ConnectionGuard {
    id: ConnectionID,
    server_protocol: ServerProtocol,
    server_socket_type: ServerSocketType,
    #[getset(skip)]
    num_requests: Arc<AtomicUsize>,
}

impl ConnectionGuard {
    fn new(
        id: ConnectionID,
        server_protocol: ServerProtocol,
        server_socket_type: ServerSocketType,
        num_requests: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            id,
            server_protocol,
            server_socket_type,
            num_requests,
        }
    }

    pub fn increment_num_requests(&self) {
        self.num_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn num_requests(&self) -> usize {
        self.num_requests.load(Ordering::Relaxed)
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let id = self.id;

        tokio::task::spawn(async move {
            ConnectionTracker::instance()
                .await
                .remove_connection(id)
                .await;
        });
    }
}

pub struct ConnectionTracker {
    state: RwLock<internal::ConnectionTrackerState>,
}

impl ConnectionTracker {
    async fn new() -> Self {
        Self {
            state: RwLock::new(internal::ConnectionTrackerState::new()),
        }
    }

    pub async fn add_connection(
        &self,
        server_protocol: ServerProtocol,
        server_socket_type: ServerSocketType,
    ) -> Option<ConnectionGuard> {
        let mut state = self.state.write().await;

        state.add_connection(server_protocol, server_socket_type)
    }

    async fn remove_connection(&self, connection_id: ConnectionID) {
        let mut state = self.state.write().await;

        state.remove_connection(connection_id);
    }

    pub async fn state(&self) -> ConnectionTrackerState {
        let state = self.state.read().await;

        ConnectionTrackerState {
            max_open_connections: state.max_open_connections(),
            connection_limit_hits: state.connection_limit_hits(),
            max_connection_age: state.max_connection_age(),
            max_requests_per_connection: state.max_requests_per_connection(),
            open_connections: state.open_connections().cloned().collect(),
        }
    }

    pub async fn instance() -> &'static Self {
        static INSTANCE: OnceCell<ConnectionTracker> = OnceCell::const_new();

        INSTANCE.get_or_init(Self::new).await
    }
}

pub struct ConnectionTrackerState {
    pub max_open_connections: usize,
    pub connection_limit_hits: usize,
    pub max_connection_age: Duration,
    pub max_requests_per_connection: usize,
    pub open_connections: Vec<ConnectionInfo>,
}
