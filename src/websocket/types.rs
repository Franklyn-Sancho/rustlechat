use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use uuid::Uuid;

use crate::models::invitation::InvitationNotification;

use super::connection_manager::ConnectionManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebSocketMessage {
    Response(ChatMessageResponse),
    Chat(ChatMessage),
    Status(StatusMessage),
    Error(ErrorMessage),
    Invitation(InvitationNotification),
}



/* #[derive(Clone)]
pub struct AppState {
    pub connections: ConnectionManager,
    pub db: Arc<Client>,
    pub auth_service: Arc<AuthService>,
    pub current_user_id: Option<Uuid>,
} */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub message_id: Uuid,
    pub chat_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessageResponse {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusMessage {
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub status: UserStatus,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserStatus {
    Online,
    Offline,
    Typing,
    Idle,
    Joined
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
}