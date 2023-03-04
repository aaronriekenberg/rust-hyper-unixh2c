use getset::Getters;

use std::sync::Arc;

use crate::connection::ConnectionTracker;

#[derive(Getters)]
#[getset(get = "pub")]
pub struct AppContext {
    connection_tracker: Arc<ConnectionTracker>,
}

impl AppContext {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connection_tracker: ConnectionTracker::new(),
        })
    }
}
