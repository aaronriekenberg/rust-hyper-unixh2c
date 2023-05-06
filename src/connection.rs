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

use std::{cmp, time::Duration};

use crate::config::ServerProtocol;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionID(pub usize);

#[derive(Clone, Debug, Getters)]
#[getset(get = "pub")]
pub struct ConnectionInfo {
    id: ConnectionID,
    creation_time: SystemTime,
    server_protocol: ServerProtocol,
    #[getset(skip)]
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

    pub fn num_requests(&self) -> usize {
        self.num_requests.load(Ordering::Relaxed)
    }

    pub fn age(&self) -> Duration {
        self.creation_time().elapsed().unwrap_or_default()
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

struct InternalConnectionTrackerState {
    next_connection_id: usize,
    max_open_connections: usize,
    past_max_connection_lifetime: Duration,
    past_max_requests_per_connection: usize,
    id_to_connection_info: HashMap<ConnectionID, ConnectionInfo>,
}

impl InternalConnectionTrackerState {
    fn new() -> Self {
        Self {
            next_connection_id: 1,
            max_open_connections: 0,
            past_max_connection_lifetime: Duration::from_secs(0),
            past_max_requests_per_connection: 0,
            id_to_connection_info: HashMap::new(),
        }
    }

    fn next_connection_id(&mut self) -> ConnectionID {
        let connection_id = self.next_connection_id;
        self.next_connection_id += 1;
        ConnectionID(connection_id)
    }

    fn max_connection_lifetime(&self) -> Duration {
        cmp::max(
            self.past_max_connection_lifetime,
            self.id_to_connection_info
                .values()
                .map(|c| c.age())
                .max()
                .unwrap_or_default(),
        )
    }

    fn max_requests_per_connection(&self) -> usize {
        cmp::max(
            self.past_max_requests_per_connection,
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
    fn new() -> Self {
        Self {
            state: RwLock::new(InternalConnectionTrackerState::new()),
        }
    }

    pub async fn add_connection(&'static self, server_protocol: ServerProtocol) -> ConnectionGuard {
        let mut state = self.state.write().await;

        let connection_id = state.next_connection_id();

        let connection_info = ConnectionInfo::new(connection_id, server_protocol);

        let num_requests = Arc::clone(&connection_info.num_requests);

        state
            .id_to_connection_info
            .insert(connection_id, connection_info);

        let num_connections = state.id_to_connection_info.len();

        state.max_open_connections = cmp::max(state.max_open_connections, num_connections);

        debug!("add_new_connection num_connections = {}", num_connections,);

        ConnectionGuard::new(self, connection_id, num_requests)
    }

    async fn remove_connection(&self, connection_id: ConnectionID) {
        let mut state = self.state.write().await;

        if let Some(connection_info) = state.id_to_connection_info.remove(&connection_id) {
            state.past_max_connection_lifetime =
                cmp::max(state.past_max_connection_lifetime, connection_info.age());

            state.past_max_requests_per_connection = cmp::max(
                state.past_max_requests_per_connection,
                connection_info.num_requests(),
            );
        }

        debug!(
            "remove_connection id_to_connection_info.len = {}",
            state.id_to_connection_info.len()
        );
    }

    pub async fn state(&self) -> ConnectionTrackerState {
        let state = self.state.read().await;

        ConnectionTrackerState {
            max_open_connections: state.max_open_connections,
            max_connection_lifetime: state.max_connection_lifetime(),
            max_requests_per_connection: state.max_requests_per_connection(),
            open_connections: state.id_to_connection_info.values().cloned().collect(),
        }
    }

    pub async fn instance() -> &'static ConnectionTracker {
        static INSTANCE: OnceCell<ConnectionTracker> = OnceCell::const_new();

        INSTANCE
            .get_or_init(|| async move { ConnectionTracker::new() })
            .await
    }
}

pub struct ConnectionTrackerState {
    pub max_open_connections: usize,
    pub max_connection_lifetime: Duration,
    pub max_requests_per_connection: usize,
    pub open_connections: Vec<ConnectionInfo>,
}
