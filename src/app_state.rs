// app_state.rs

use deadpool_postgres::Pool;
use std::sync::Arc;
use uuid::Uuid;
use crate::websocket::connection_manager::ConnectionManager;

/// Application state containing shared resources
#[derive(Clone)]
pub struct AppState {
    /// WebSocket connection manager
    pub connections: ConnectionManager,
    /// Database connection pool wrapped in Arc for thread-safe sharing
    pub db: Arc<Pool>,
    /// Optional ID of the currently authenticated user
    pub current_user_id: Option<Uuid>,
}

impl AppState {
    /// Creates a new instance of AppState
    /// 
    /// # Arguments
    /// * `db` - Arc-wrapped database connection pool
    /// * `connections` - WebSocket connection manager
    /// 
    /// # Returns
    /// * `Self` - New AppState instance
    pub fn new(db: Arc<Pool>, connections: ConnectionManager) -> Self {
        Self {
            connections,
            db,
            current_user_id: None,
        }
    }
}
