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

async fn get_or_create_chat_key(db_pool: &Pool, chat_id: Uuid, user_id: Uuid) -> Result<ChatKey, String> {
    let mut client = db_pool.get().await.map_err(|e| format!("DB error: {}", e))?;
    let transaction = client.transaction().await.map_err(|e| e.to_string())?;

    if let Some(key) = ChatRepository::get_chat_key(&transaction, chat_id, user_id).await? {
        Ok(ChatKey(key))
    } else {
        let chat_key = MessageCrypto::generate_chat_key();
        ChatRepository::store_chat_key(&transaction, chat_id, user_id, chat_key.0.clone()).await?;
        transaction.commit().await.map_err(|e| e.to_string())?;
        Ok(chat_key)
    }
}

/// Handles a WebSocket connection
pub async fn handle_websocket_connection(
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
            log::error!("Failed to add user to chat: {}", e);
            return;
        }
    };

    // Get or create chat key
    let chat_key = match get_or_create_chat_key(&db_pool, chat_id, user_id).await {
        Ok(key) => key,
        Err(e) => {
            log::error!("Failed to get chat key: {}", e);
            return;
        }
    };

    // Initialize message crypto
    let message_crypto = match MessageCrypto::new(&chat_key) {
        Ok(mc) => mc,
        Err(e) => {
            log::error!("Failed to initialize crypto: {}", e);
            return;
        }
    };

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = handle_incoming_message(
                            &conn_manager,
                            &db_pool,
                            &message_crypto,
                            chat_id,
                            user_id,
                            text,
                        ).await {
                            log::error!("Error handling incoming message: {}", e);
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
                                log::error!("Send error: {}", e);
                                break;
                            }
                        }
                        Err(e) => log::error!("Decryption error: {}", e),
                    }
                }
            }
        }
    }
}

/// Handles an incoming WebSocket message
async fn handle_incoming_message(
    conn_manager: &ConnectionManager,
    db_pool: &Pool,
    message_crypto: &MessageCrypto,
    chat_id: Uuid,
    user_id: Uuid,
    text: String,
) -> Result<(), String> {
    // Encrypt message
    let encrypted = message_crypto.encrypt(&text).map_err(|e| e.to_string())?;

    // Generate a new UUID for the message
    let message_id = Uuid::new_v4();

    // Insert the encrypted message into the database
    let mut client = db_pool.get().await.map_err(|e| format!("Failed to get DB client: {}", e))?;
    let transaction = client.transaction().await.map_err(|e| format!("Failed to start transaction: {}", e))?;

    ChatRepository::insert_encrypted_message(
        &transaction,
        message_id,
        chat_id,
        user_id,
        &encrypted,
    ).await?;

    transaction.commit().await.map_err(|e| format!("Failed to commit transaction: {}", e))?;

    // Broadcast the message to other users in the chat
    let chat_msg = WebSocketMessage::Chat(ChatMessage {
        message_id,
        chat_id,
        sender_id: user_id,
        content: encrypted,
        timestamp: Utc::now().naive_utc(),
    });

    conn_manager.broadcast_message(chat_msg, chat_id, user_id)?;

    Ok(())
}




