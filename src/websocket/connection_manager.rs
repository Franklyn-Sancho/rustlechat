use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;
use uuid::Uuid;
use deadpool_postgres::Pool;

use super::types::{UserStatus, WebSocketMessage};

// Represents an online user
#[derive(Debug, Clone)]
pub struct OnlineUser {
    pub id: Uuid, // User ID
    pub username: String, // Username
    pub sender: broadcast::Sender<WebSocketMessage>, // Sender for WebSocket messages
}

// Connection manager for handling active chats and user connections
#[derive(Clone)]
pub struct ConnectionManager {
    pub chats: Arc<Mutex<HashMap<Uuid, ChatRoom>>>, // Maps chat IDs to chat rooms
    pub connections: Arc<RwLock<HashMap<Uuid, OnlineUser>>>, // Maps user IDs to online users
    pub usernames: Arc<RwLock<HashMap<String, Uuid>>>, // Maps usernames to user IDs
    pub db_pool: Pool, // Database connection pool (agora é Pool diretamente)
}

// Represents a chat room
pub struct ChatRoom {
    pub users: HashMap<Uuid, UserConnection>, // Maps user IDs to user connections within a chat
    pub channel: broadcast::Sender<WebSocketMessage>, // Channel for broadcasting messages
}

// Represents a user's connection within a chat
pub struct UserConnection {
    pub sender: broadcast::Sender<WebSocketMessage>, // Sender for WebSocket messages
    pub status: UserStatus, // User's status (online/offline)
    pub last_activity: chrono::DateTime<chrono::Utc>, // Timestamp of the last activity
}

impl ConnectionManager {
    // Creates a new ConnectionManager instance
    pub fn new(db_pool: Pool) -> Self {
        Self {
            chats: Arc::new(Mutex::new(HashMap::new())), // Initialize empty chats
            connections: Arc::new(RwLock::new(HashMap::new())), // Initialize empty connections
            usernames: Arc::new(RwLock::new(HashMap::new())), // Initialize empty usernames
            db_pool, // Initialize the database pool (agora é Pool diretamente)
        }
    }

    // Adds a user to a chat room and returns a message receiver for WebSocket communication
    pub async fn add_user_to_chat(
        &self,
        chat_id: Uuid, // The ID of the chat
        user_id: Uuid, // The ID of the user
    ) -> Result<broadcast::Receiver<WebSocketMessage>, String> {
        log::info!("Adding user {} to chat {}", user_id, chat_id);
        
        let mut chats = self.chats.lock().map_err(|e| {
            log::error!("Error locking chats: {}", e);
            format!("Failed to lock chat rooms: {}", e)
        })?;
    
        let chat_room = chats.entry(chat_id).or_insert_with(|| {
            log::info!("Creating new chat room {}", chat_id);
            let (tx, _) = broadcast::channel(100); // Create a new broadcast channel for the chat
            ChatRoom {
                users: HashMap::new(), // Initialize empty users list
                channel: tx,
            }
        });
    
        // If the user is not already in the chat, add them
        if !chat_room.users.contains_key(&user_id) {
            log::info!("Adding new user {} to chat {}", user_id, chat_id);
            let (tx, _) = broadcast::channel(100); // Create a new sender for the user
            let user_conn = UserConnection {
                sender: tx,
                status: UserStatus::Online, // Set user status to online
                last_activity: Utc::now(),
            };
            chat_room.users.insert(user_id, user_conn); // Insert user into the chat
        }
    
        log::info!("User {} successfully added to chat {}", user_id, chat_id);
        Ok(chat_room.channel.subscribe()) // Return the receiver to listen for messages
    }

    // Removes a user from a chat room
    pub fn remove_user_from_chat(&self, chat_id: Uuid, user_id: Uuid) -> Result<(), String> {
        let mut chats = self.chats.lock().map_err(|_| "Failed to lock chat rooms")?;

        if let Some(chat_room) = chats.get_mut(&chat_id) {
            chat_room.users.remove(&user_id); // Remove user from chat

            // If no users are left in the chat, remove the chat room itself
            if chat_room.users.is_empty() {
                chats.remove(&chat_id);
            }
        }

        Ok(())
    }

    // Broadcasts a message to all users in a specific chat room
    pub fn broadcast_message(
        &self,
        message: WebSocketMessage, // The message to broadcast
        chat_id: Uuid, // The ID of the chat room
        sender_id: Uuid, // The ID of the user sending the message
    ) -> Result<(), String> {
        let chats = self.chats.lock().map_err(|_| "Failed to lock chat rooms")?;

        if let Some(chat_room) = chats.get(&chat_id) {
            let _ = chat_room.channel.send(message); // Send the message to all users in the chat
        }

        Ok(())
    }

    // Broadcasts a message to a chat room asynchronously
    pub async fn broadcast_to_chat(
        &self,
        chat_id: Uuid, // The ID of the chat room
        sender_id: Uuid, // The ID of the user sending the message
        message: WebSocketMessage, // The message to broadcast
    ) -> Result<(), String> {
        let chats = self.chats.lock().map_err(|_| "Failed to lock chat rooms")?;

        if let Some(chat_room) = chats.get(&chat_id) {
            let _ = chat_room.channel.send(message); // Send the message to all users in the chat
        }

        Ok(())
    }

    // Updates the status of a user in a chat room (e.g., online/offline)
    pub fn update_user_status(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
        status: UserStatus, // The new status for the user
    ) -> Result<(), String> {
        let mut chats = self.chats.lock().map_err(|_| "Failed to lock chat rooms")?;

        if let Some(chat_room) = chats.get_mut(&chat_id) {
            if let Some(user_conn) = chat_room.users.get_mut(&user_id) {
                user_conn.status = status; // Update the user's status
                user_conn.last_activity = chrono::Utc::now(); // Update the last activity timestamp
            }
        }

        Ok(())
    }

    // Sends a direct message to a specific user
    pub async fn send_direct_message(
        &self,
        user_id: Uuid, // The ID of the user to send the message to
        message: WebSocketMessage, // The message to send
    ) -> Result<(), String> {
        let connections = self.connections.read().map_err(|_| "Failed to lock user connections")?;

        if let Some(user) = connections.get(&user_id) {
            user.sender
                .send(message) // Send the message to the user's sender
                .map_err(|_| "Failed to send message")?;
            Ok(())
        } else {
            Err("User not found".to_string()) // Return error if user not found
        }
    }

    // Retrieves an online user by their username
    pub async fn get_online_user(&self, username: &str) -> Option<OnlineUser> {
        let usernames = self.usernames.read().ok()?; // Get the usernames map
        let user_id = usernames.get(username)?; // Find the user ID by username

        let connections = self.connections.read().ok()?; // Get the connections map
        connections.get(user_id).cloned() // Return the user's details
    }

    // Example function to interact with the DB
    pub async fn get_user_from_db(&self, user_id: Uuid) -> Result<String, String> {
        let client = self.db_pool.get().await.map_err(|e| format!("Error getting client from pool: {}", e))?;
        let stmt = client.prepare("SELECT username FROM users WHERE id = $1").await.map_err(|e| e.to_string())?;
        let row = client.query_one(&stmt, &[&user_id]).await.map_err(|e| e.to_string())?;
        Ok(row.get(0))
    }
}


