use std::sync::{Arc, Mutex};

use tokio_postgres::Client;
use uuid::Uuid;

use crate::websocket::connection_manager::ConnectionManager;


#[derive(Clone)]
pub struct AppState {
    pub connections: ConnectionManager, // Gerenciador de conexões
    pub db: Arc<Client>,                // Cliente do banco de dados
    pub current_user_id: Option<Uuid>,  // ID do usuário atual (após autenticação)
}

impl AppState {
    // Construtor para criar uma nova instância de AppState
    pub fn new(db: Arc<Client>, connections: ConnectionManager) -> Self {
        Self {
            connections,
            db,
            current_user_id: None,
        }
    }
}

// Define um tipo compartilhado para o AppState (envolvido em Arc e Mutex)
pub type SharedAppState = Arc<Mutex<AppState>>;