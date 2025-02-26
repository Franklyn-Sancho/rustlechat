use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
    Extension,
};
use chrono::Utc;
use hyper::{HeaderMap, StatusCode};
use uuid::Uuid;
use deadpool_postgres::{Pool, Client};
use crate::{
    app_state::AppState, crypto::{ChatKey, MessageCrypto}, middleware::ws_auth_middleware::WebSocketParams, repositories::chat_repository::ChatRepository, services::jwt_service::validate_token, websocket::types::ChatMessage
};

use super::{
    connection_manager::ConnectionManager,
    types::{StatusMessage, UserStatus, WebSocketMessage},
};

// Handles the initial WebSocket upgrade request
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    Extension(state): Extension<AppState>,
    Query(params): Query<WebSocketParams>,
) -> impl IntoResponse {
    // Check if the 'Authorization' header is present
    if let Some(authorization) = headers.get("Authorization") {
        if let Ok(token) = authorization.to_str() {
            if let Some(user_id) = validate_token(token) {
                return ws.on_upgrade(move |socket| {
                    handle_websocket_connection(socket, state, params.chat_id, user_id)
                });
            }
        }
    }

    StatusCode::UNAUTHORIZED.into_response()
}

// Checks if the user is allowed to send messages in the specified chat
async fn can_user_send_message(
    connection_manager: &ConnectionManager,
    db_pool: &Pool,
    chat_id: Uuid,
    user_id: Uuid,
) -> Result<bool, String> {
    log::info!(
        "Checking if user with ID {} can send messages in chat with ID {}",
        user_id,
        chat_id
    );

    // Get a database connection from the pool
    let client = db_pool.get().await.map_err(|e| format!("Error getting DB client: {}", e))?;

    // SQL query to check if the user is an accepted member or the creator of the chat
    let query = "
        SELECT 1 FROM chat_members 
        WHERE chat_id = $1 AND user_id = $2 
        AND (status = 'accepted' OR is_creator = true)
    ";

    match client.query_opt(query, &[&chat_id, &user_id]).await {
        Ok(Some(_)) => {
            log::info!("User with ID {} is authorized in chat ID {}", user_id, chat_id);

            // If authorized in the DB, check if the user is already added in memory
            if !is_user_in_chat(connection_manager, chat_id, user_id).await? {
                // If not, add the user to the chat in the ConnectionManager
                let _ = connection_manager.add_user_to_chat(chat_id, user_id).await?;
            }
            Ok(true)
        }
        Ok(None) => {
            log::warn!("User with ID {} is not authorized in chat ID {}", user_id, chat_id);
            Err("User is not authorized in this chat".to_string())
        }
        Err(e) => {
            log::error!("Error checking authorization: {}", e);
            Err(format!("Error checking authorization: {}", e))
        }
    }
}

async fn get_or_create_chat_key(db_pool: &Pool, chat_id: Uuid, user_id: Uuid) -> Result<ChatKey, String> {
    let mut client = db_pool.get().await.map_err(|e| format!("DB error: {}", e))?;
    let transaction = client.transaction().await.map_err(|e| e.to_string())?;

    // Try to get existing key
    if let Some(key) = ChatRepository::get_chat_key(&transaction, chat_id, user_id).await
        .map_err(|e| e.to_string())? {
        return Ok(ChatKey(key));
    }

    // Generate new key if none exists
    let chat_key = MessageCrypto::generate_chat_key();
    ChatRepository::store_chat_key(&transaction, chat_id, user_id, chat_key.0.clone())
        .await
        .map_err(|e| e.to_string())?;

    transaction.commit().await.map_err(|e| e.to_string())?;
    Ok(chat_key)
}

// Handles the WebSocket connection once it has been upgraded
async fn handle_websocket_connection(
    mut socket: WebSocket,
    state: AppState,
    chat_id: Uuid,
    user_id: Uuid,
) {
    let conn_manager = &state.connections;
    let db_pool = state.db.clone();

    // Add user to chat and get receiver
    let mut rx = match conn_manager.add_user_to_chat(chat_id, user_id).await {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("Failed to add user to chat: {}", e);
            return;
        }
    };

    // Get or create chat key
    let chat_key = match get_or_create_chat_key(&db_pool, chat_id, user_id).await {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Failed to get chat key: {}", e);
            return;
        }
    };

    // Initialize message crypto
    let message_crypto = match MessageCrypto::new(&chat_key) {
        Ok(mc) => mc,
        Err(e) => {
            eprintln!("Failed to initialize crypto: {}", e);
            return;
        }
    };

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Encrypt message
                        let encrypted = match message_crypto.encrypt(&text) {
                            Ok(enc) => enc,
                            Err(e) => {
                                eprintln!("Encryption error: {}", e);
                                continue;
                            }
                        };

                        let chat_msg = WebSocketMessage::Chat(ChatMessage {
                            message_id: Uuid::new_v4(),
                            chat_id,
                            sender_id: user_id,
                            content: encrypted,
                            timestamp: Utc::now().naive_utc(),
                        });

                        if let Err(e) = conn_manager.broadcast_message(chat_msg, chat_id, user_id) {
                            eprintln!("Failed to broadcast: {}", e);
                            break;
                        }
                    }
                    _ => continue,
                }
            }
            
            Ok(msg) = rx.recv() => {
                if let WebSocketMessage::Chat(chat_msg) = msg {
                    match message_crypto.decrypt(&chat_msg.content) {
                        Ok(decrypted) => {
                            if let Err(e) = socket.send(Message::Text(decrypted)).await {
                                eprintln!("Send error: {}", e);
                                break;
                            }
                        }
                        Err(e) => eprintln!("Decryption error: {}", e)
                    }
                }
            }
        }
    }
}

/// RAII guard to ensure proper cleanup of user connection
struct CleanupGuard {
    conn_manager: ConnectionManager,
    chat_id: Uuid,
    user_id: Uuid,
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = self.conn_manager.update_user_status(
            self.chat_id,
            self.user_id,
            UserStatus::Offline
        );
        let _ = self.conn_manager.remove_user_from_chat(
            self.chat_id,
            self.user_id
        );
    }
}

// Helper function to check if a user is already present in the in-memory representation of a chat
pub async fn is_user_in_chat(
    connection_manager: &ConnectionManager,
    chat_id: Uuid,
    user_id: Uuid,
) -> Result<bool, String> {
    let chats = connection_manager.chats.lock().map_err(|_| "Lock error")?;
    Ok(chats
        .get(&chat_id)
        .map(|chat| chat.users.contains_key(&user_id))
        .unwrap_or(false))
}
