/* use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::extract::ws::Message;
use axum::{
    extract::{ws::WebSocket, Query, WebSocketUpgrade},
    response::{IntoResponse, Response},
    Extension, Json,
};
use hyper::StatusCode;
use tokio::sync::broadcast;
use tokio_postgres::Client;
use uuid::Uuid;

use crate::{middleware::ws_auth_middleware::WebSocketParams, services::jwt_service::validate_token};

#[derive(Clone)]
pub struct AppState {
    pub connections: Connections,  // Manages WebSocket connections
    pub db: Arc<Client>,  // Database connection
    pub current_user_id: Option<Uuid>,  // Currently authenticated user's ID
}

#[derive(Clone)]
pub struct Connections {
    // Maps a chat_id to another map that associates user_id with a message sender
    pub chats: Arc<Mutex<HashMap<Uuid, HashMap<Uuid, broadcast::Sender<String>>>>>,
}

impl Connections {
    // Creates a new instance of the Connections struct
    pub fn new() -> Self {
        Self {
            chats: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Adds a user to a chat and returns a receiver to listen for messages
    pub fn add_user_to_chat(&self, chat_id: Uuid, user_id: Uuid) -> broadcast::Receiver<String> {
        let mut chats = self.chats.lock().unwrap();
        
        // If the chat does not exist, create a new entry
        let chat_users = chats.entry(chat_id)
            .or_insert_with(HashMap::new);
        
        // If the user is not in the chat, create a new broadcast channel
        if !chat_users.contains_key(&user_id) {
            let (tx, rx) = broadcast::channel(32);
            chat_users.insert(user_id, tx);
            rx
        } else {
            // If the user is already in the chat, return a new subscriber
            chat_users.get(&user_id).unwrap().subscribe()
        }
    }

    // Removes a user from a chat
    pub fn remove_user_from_chat(&self, chat_id: Uuid, user_id: Uuid) {
        let mut chats = self.chats.lock().unwrap();
        if let Some(chat_users) = chats.get_mut(&chat_id) {
            chat_users.remove(&user_id);
            
            // If there are no more users in the chat, remove the chat itself
            if chat_users.is_empty() {
                chats.remove(&chat_id);
            }
        }
    }

    // Broadcasts a message to all users in a chat except the sender
    pub fn broadcast_to_chat(&self, chat_id: Uuid, sender_id: Uuid, message: String) {
        let chats = self.chats.lock().unwrap();
        if let Some(chat_users) = chats.get(&chat_id) {
            for (user_id, tx) in chat_users.iter() {
                if *user_id != sender_id {  // Do not send the message back to the sender
                    let _ = tx.send(message.clone());
                }
            }
        }
    }
}


pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<AppState>,
    Query(params): Query<WebSocketParams>,
) -> Response {
    match params.token {
        Some(token) => {
            if let Some(user_id) = validate_token(&token) {
                // Uses `move` to ensure values are moved into the closure
                return ws.on_upgrade(move |socket| handle_websocket(socket, state, params.chat_id, user_id));
            }
        }
        None => {
            eprintln!("Missing or invalid token");
        }
    }

    StatusCode::UNAUTHORIZED.into_response()
}

pub async fn handle_websocket(
    mut socket: WebSocket,
    state: AppState,
    chat_id: Uuid,
    user_id: Uuid,
) {
    println!("User {} joined chat {}", user_id, chat_id);
    
    // Adiciona o usuário ao chat e obtém um receiver
    let mut rx = state.connections.add_user_to_chat(chat_id, user_id);

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(msg)) => {
                        if let Ok(text) = msg.to_text() {
                            println!("User {} sent message in chat {}: {}", user_id, chat_id, text);
                            // Transmite a mensagem para todos os outros usuários no chat
                            state.connections.broadcast_to_chat(chat_id, user_id, text.to_string());
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket error for user {}: {}", user_id, e);
                        break;
                    }
                    None => {
                        println!("WebSocket closed for user {}", user_id);
                        break;
                    }
                }
            }
            Ok(msg) = rx.recv() => {
                if let Err(e) = socket.send(Message::Text(msg)).await {
                    eprintln!("Error sending message to user {}: {}", user_id, e);
                    break;
                }
            }
        }
    }

    // Limpa a conexão quando o usuário sai
    state.connections.remove_user_from_chat(chat_id, user_id);
    println!("User {} left chat {}", user_id, chat_id);
} */
