// src/websocket/connection_manager.rs
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;
use std::collections::HashMap;

use super::types::{UserStatus, WebSocketMessage};

#[derive(Clone)]
pub struct ConnectionManager {
    chats: Arc<Mutex<HashMap<Uuid, ChatRoom>>>,
}

struct ChatRoom {
    users: HashMap<Uuid, UserConnection>,
    channel: broadcast::Sender<WebSocketMessage>,
}

struct UserConnection {
    sender: broadcast::Sender<WebSocketMessage>,
    status: UserStatus,
    last_activity: chrono::DateTime<chrono::Utc>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            chats: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_user_to_chat(&self, chat_id: Uuid, user_id: Uuid) -> Result<broadcast::Receiver<WebSocketMessage>, String> {
        let mut chats = self.chats.lock().map_err(|_| "Lock error")?;
        
        let chat_room = chats.entry(chat_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            ChatRoom {
                users: HashMap::new(),
                channel: tx,
            }
        });

        if !chat_room.users.contains_key(&user_id) {
            let (tx, _) = broadcast::channel(100);
            let user_conn = UserConnection {
                sender: tx,
                status: UserStatus::Online,
                last_activity: chrono::Utc::now(),
            };
            chat_room.users.insert(user_id, user_conn);
        }

        Ok(chat_room.channel.subscribe())
    }

    pub fn remove_user_from_chat(&self, chat_id: Uuid, user_id: Uuid) -> Result<(), String> {
        let mut chats = self.chats.lock().map_err(|_| "Lock error")?;
        
        if let Some(chat_room) = chats.get_mut(&chat_id) {
            chat_room.users.remove(&user_id);
            
            if chat_room.users.is_empty() {
                chats.remove(&chat_id);
            }
        }

        Ok(())
    }

    pub fn broadcast_message(&self, message: WebSocketMessage, chat_id: Uuid, sender_id: Uuid) -> Result<(), String> {
        let chats = self.chats.lock().map_err(|_| "Lock error")?;
        
        if let Some(chat_room) = chats.get(&chat_id) {
            let _ = chat_room.channel.send(message);
        }

        Ok(())
    }

    pub fn update_user_status(&self, chat_id: Uuid, user_id: Uuid, status: UserStatus) -> Result<(), String> {
        let mut chats = self.chats.lock().map_err(|_| "Lock error")?;
        
        if let Some(chat_room) = chats.get_mut(&chat_id) {
            if let Some(user_conn) = chat_room.users.get_mut(&user_id) {
                user_conn.status = status;
                user_conn.last_activity = chrono::Utc::now();
            }
        }

        Ok(())
    }
}