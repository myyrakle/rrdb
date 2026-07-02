use std::sync::Arc;

use crate::engine::{DBEngine, SharedWALManager};

use super::client::ClientInfo;

#[derive(Clone)]
pub struct SharedState {
    pub engine: Arc<DBEngine>,
    pub wal_manager: SharedWALManager,
    pub client_info: ClientInfo,
}
