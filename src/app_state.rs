// app_state.rs
use deadpool_postgres::Pool;
use std::sync::Arc;
use uuid::Uuid;
use crate::websocket::connection_manager::ConnectionManager;
use crate::crypto::MessageCrypto;

/// Application state containing shared resources
#[derive(Clone)]
pub struct AppState {
    pub connections: ConnectionManager,
    pub db: Pool,
    pub crypto: Arc<MessageCrypto>,
    pub current_user_id: Option<Uuid>,
}

impl AppState {
    /// Creates a new instance of AppState
    ///
    /// # Arguments
    /// * `db` - Arc-wrapped database connection pool
    /// * `connections` - WebSocket connection manager
    /// * `crypto` - Arc-wrapped MessageCrypto instance
    ///
    /// # Returns
    /// * `Self` - New AppState instance
    pub fn new(db: Pool, connections: ConnectionManager, crypto: Arc<MessageCrypto>) -> Self {
        Self {
            connections,
            db,
            crypto,
            current_user_id: None,
        }
    }
}
