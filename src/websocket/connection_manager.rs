// src/websocket/connection_manager.rs
use chrono::Utc;
use std::collections::HashMap;
use std::{
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::database::init::DbClient;

use super::types::{UserStatus, WebSocketMessage};

#[derive(Debug, Clone)]
pub struct OnlineUser {
    pub id: Uuid,
    pub username: String,
    pub sender: broadcast::Sender<WebSocketMessage>,
}

#[derive(Clone)]
pub struct ConnectionManager {
    pub chats: Arc<Mutex<HashMap<Uuid, ChatRoom>>>,
    pub connections: Arc<RwLock<HashMap<Uuid, OnlineUser>>>,
    pub usernames: Arc<RwLock<HashMap<String, Uuid>>>,
}

pub struct ChatRoom {
    pub users: HashMap<Uuid, UserConnection>,
    pub channel: broadcast::Sender<WebSocketMessage>,
}

pub struct UserConnection {
    pub sender: broadcast::Sender<WebSocketMessage>,
    pub status: UserStatus,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            chats: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            usernames: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_user_to_chat(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<broadcast::Receiver<WebSocketMessage>, String> {
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
                last_activity: Utc::now(),
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

    pub fn broadcast_message(
        &self,
        message: WebSocketMessage,
        chat_id: Uuid,
        sender_id: Uuid,
    ) -> Result<(), String> {
        let chats = self.chats.lock().map_err(|_| "Lock error")?;

        if let Some(chat_room) = chats.get(&chat_id) {
            let _ = chat_room.channel.send(message);
        }

        Ok(())
    }

    pub async fn broadcast_to_chat(
        &self,
        chat_id: Uuid,
        sender_id: Uuid,
        message: WebSocketMessage,
    ) -> Result<(), String> {
        let chats = self.chats.lock().map_err(|_| "Lock error")?;

        if let Some(chat_room) = chats.get(&chat_id) {
            let _ = chat_room.channel.send(message);
        }

        Ok(())
    }

    pub fn update_user_status(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
        status: UserStatus,
    ) -> Result<(), String> {
        let mut chats = self.chats.lock().map_err(|_| "Lock error")?;

        if let Some(chat_room) = chats.get_mut(&chat_id) {
            if let Some(user_conn) = chat_room.users.get_mut(&user_id) {
                user_conn.status = status;
                user_conn.last_activity = chrono::Utc::now();
            }
        }

        Ok(())
    }

    pub async fn send_direct_message(
        &self,
        user_id: Uuid,
        message: WebSocketMessage,
    ) -> Result<(), String> {
        let connections = self.connections.read().map_err(|_| "Lock error")?;

        if let Some(user) = connections.get(&user_id) {
            user.sender
                .send(message)
                .map_err(|_| "Failed to send message")?;
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn get_online_user(&self, username: &str) -> Option<OnlineUser> {
        // Primeiro tentamos obter o UUID do usuário pelo username
        let usernames = self.usernames.read().ok()?;
        let user_id = usernames.get(username)?;

        // Então buscamos os detalhes da conexão
        let connections = self.connections.read().ok()?;
        connections.get(user_id).cloned()
    }

    // Método auxiliar para adicionar uma nova conexão
    pub async fn add_connection(
        &self,
        user_id: Uuid,
        username: String,
    ) -> Result<broadcast::Receiver<WebSocketMessage>, String> {
        let (sender, receiver) = broadcast::channel(100);

        let user = OnlineUser {
            id: user_id,
            username: username.clone(),
            sender,
        };

        // Atualiza os hashmaps de conexão
        {
            let mut connections = self.connections.write().map_err(|_| "Lock error")?;
            let mut usernames = self.usernames.write().map_err(|_| "Lock error")?;

            connections.insert(user_id, user);
            usernames.insert(username, user_id);
        }

        Ok(receiver)
    }

    // Método auxiliar para remover uma conexão
    pub async fn remove_connection(&self, user_id: Uuid) -> Result<(), String> {
        let mut connections = self.connections.write().map_err(|_| "Lock error")?;

        if let Some(user) = connections.remove(&user_id) {
            let mut usernames = self.usernames.write().map_err(|_| "Lock error")?;
            usernames.remove(&user.username);

            // Remove o usuário de todos os chats
            let mut chats = self.chats.lock().map_err(|_| "Lock error")?;
            for chat_room in chats.values_mut() {
                chat_room.users.remove(&user_id);
            }
        }

        Ok(())
    }
}
