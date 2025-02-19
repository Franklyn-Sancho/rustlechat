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
    app_state::AppState,
    middleware::ws_auth_middleware::WebSocketParams,
    services::jwt_service::validate_token,
    websocket::types::ChatMessage,
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

// Handles the WebSocket connection once it has been upgraded
async fn handle_websocket_connection(
    mut socket: WebSocket,
    state: AppState,
    chat_id: Uuid,
    user_id: Uuid,
) {
    let conn_manager = &state.connections;
    let db_pool = state.db.clone(); // Clone the DB pool from state

    // Verify if the user is allowed to send messages in the chat
    if let Err(e) = can_user_send_message(conn_manager, &db_pool, chat_id, user_id).await {
        eprintln!("{}", e);
        return; // Close the connection if the user is not authorized
    }

    // Add the user to the chat (in memory) and get the receiver channel for broadcasted messages
    let mut rx = match conn_manager.add_user_to_chat(chat_id, user_id).await {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("Failed to add user to chat: {}", e);
            return;
        }
    };

    // Broadcast a status message to notify other users that this user is now online
    let status_msg = WebSocketMessage::Status(StatusMessage {
        chat_id,
        user_id,
        status: UserStatus::Online,
        timestamp: Utc::now().naive_utc(),
    });
    let _ = conn_manager.broadcast_message(status_msg, chat_id, user_id);

    // Main loop to handle incoming and outgoing WebSocket messages
    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(msg)) => {
                        if let Ok(text) = msg.to_text() {
                            let chat_msg = WebSocketMessage::Chat(ChatMessage {
                                message_id: Uuid::new_v4(),
                                chat_id,
                                sender_id: user_id,
                                content: text.to_string(),
                                timestamp: Utc::now().naive_utc(),
                            });

                            if let Err(e) = conn_manager.broadcast_message(chat_msg, chat_id, user_id) {
                                eprintln!("Failed to broadcast message: {}", e);
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                }
            }
            Ok(msg) = rx.recv() => {
                if let WebSocketMessage::Chat(chat_msg) = msg {
                    if let Err(e) = socket.send(Message::Text(chat_msg.content)).await {
                        eprintln!("Failed to send message: {}", e);
                        break;
                    }
                }
            }
        }
    }

    let _ = conn_manager.update_user_status(chat_id, user_id, UserStatus::Offline);
    let _ = conn_manager.remove_user_from_chat(chat_id, user_id);
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
