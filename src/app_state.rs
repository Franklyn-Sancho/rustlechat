use std::sync::{Arc, Mutex};

use tokio_postgres::Client;
use uuid::Uuid;

use crate::websocket::connection_manager::ConnectionManager;


#[derive(Clone)]
pub struct AppState {
    pub connections: ConnectionManager, 
    pub db: Arc<Client>,                
    pub current_user_id: Option<Uuid>,  
}

impl AppState {
    pub fn new(db: Arc<Client>, connections: ConnectionManager) -> Self {
        Self {
            connections,
            db,
            current_user_id: None,
        }
    }
}