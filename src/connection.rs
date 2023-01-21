use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug)]
pub struct ConnectionID(pub u64);

pub struct ConnectionIDFactory {
    next_connection_id: AtomicU64,
}

impl ConnectionIDFactory {
    pub fn new() -> Self {
        Self {
            next_connection_id: AtomicU64::new(1),
        }
    }

    pub fn new_connection_id(&self) -> ConnectionID {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);

        ConnectionID(id)
    }
}
