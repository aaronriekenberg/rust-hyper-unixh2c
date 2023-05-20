use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::SystemTime,
};

use getset::CopyGetters;

use tokio::{
    sync::{OnceCell, RwLock},
    time::{Duration, Instant},
};

use tracing::debug;

use std::cmp;

use crate::config::ServerProtocol;

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
    #[getset(skip)]
    num_requests: Arc<AtomicUsize>,
}

impl ConnectionInfo {
    fn new(id: ConnectionID, server_protocol: ServerProtocol) -> Self {
        Self {
            id,
            creation_time: SystemTime::now(),
            creation_instant: Instant::now(),
            server_protocol,
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

pub struct ConnectionGuard {
    id: ConnectionID,
    num_requests: Arc<AtomicUsize>,
}

impl ConnectionGuard {
    fn new(id: ConnectionID, num_requests: Arc<AtomicUsize>) -> Self {
        Self { id, num_requests }
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
        let id = self.id;

        tokio::task::spawn(async move {
            ConnectionTracker::instance()
                .await
                .remove_connection(id)
                .await;
        });
    }
}

#[derive(Default)]
struct InternalConnectionTrackerStateMetrics {
    max_open_connections: usize,
    past_max_connection_age: Duration,
    past_max_requests_per_connection: usize,
}

impl InternalConnectionTrackerStateMetrics {
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
struct InternalConnectionTrackerState {
    next_connection_id: usize,
    id_to_connection_info: HashMap<ConnectionID, ConnectionInfo>,
    metrics: InternalConnectionTrackerStateMetrics,
}

impl InternalConnectionTrackerState {
    fn new() -> Self {
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

    fn add_connection(&mut self, server_protocol: ServerProtocol) -> ConnectionGuard {
        let connection_id = self.next_connection_id();

        let connection_info = ConnectionInfo::new(connection_id, server_protocol);

        let num_requests = Arc::clone(&connection_info.num_requests);

        self.id_to_connection_info
            .insert(connection_id, connection_info);

        let num_connections = self.id_to_connection_info.len();

        self.metrics.update_for_new_connection(num_connections);

        debug!("add_connection num_connections = {}", num_connections);

        ConnectionGuard::new(connection_id, num_requests)
    }

    fn remove_connection(&mut self, connection_id: ConnectionID) {
        if let Some(connection_info) = self.id_to_connection_info.remove(&connection_id) {
            self.metrics.update_for_removed_connection(&connection_info);
        }

        debug!(
            "remove_connection id_to_connection_info.len = {}",
            self.id_to_connection_info.len()
        );
    }

    fn max_connection_age(&self) -> Duration {
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

    fn max_requests_per_connection(&self) -> usize {
        cmp::max(
            self.metrics.past_max_requests_per_connection,
            self.id_to_connection_info
                .values()
                .map(|c| c.num_requests())
                .max()
                .unwrap_or_default(),
        )
    }
}

pub struct ConnectionTracker {
    state: RwLock<InternalConnectionTrackerState>,
}

impl ConnectionTracker {
    async fn new() -> Self {
        Self {
            state: RwLock::new(InternalConnectionTrackerState::new()),
        }
    }

    pub async fn add_connection(&self, server_protocol: ServerProtocol) -> ConnectionGuard {
        let mut state = self.state.write().await;

        state.add_connection(server_protocol)
    }

    async fn remove_connection(&self, connection_id: ConnectionID) {
        let mut state = self.state.write().await;

        state.remove_connection(connection_id);
    }

    pub async fn state(&self) -> ConnectionTrackerState {
        let state = self.state.read().await;

        ConnectionTrackerState {
            max_open_connections: state.metrics.max_open_connections,
            max_connection_age: state.max_connection_age(),
            max_requests_per_connection: state.max_requests_per_connection(),
            open_connections: state.id_to_connection_info.values().cloned().collect(),
        }
    }

    pub async fn instance() -> &'static Self {
        static INSTANCE: OnceCell<ConnectionTracker> = OnceCell::const_new();

        INSTANCE.get_or_init(Self::new).await
    }
}

pub struct ConnectionTrackerState {
    pub max_open_connections: usize,
    pub max_connection_age: Duration,
    pub max_requests_per_connection: usize,
    pub open_connections: Vec<ConnectionInfo>,
}
