use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use uuid::Uuid;

use super::connection_manager::ConnectionManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebSocketMessage {
    Response(ChatMessageResponse),
    Chat(ChatMessage),
    Status(StatusMessage),
    Error(ErrorMessage),
}

#[derive(Clone)]
pub struct AppState {
    pub connections: ConnectionManager,
    pub db: Arc<Client>,
    pub current_user_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub message_id: Uuid,
    pub chat_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessageResponse {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusMessage {
    pub user_id: Uuid,
    pub status: UserStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserStatus {
    Online,
    Offline,
    Typing,
    Idle,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
}