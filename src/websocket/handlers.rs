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

use crate::{
    app_state::AppState,
    database::init::DbClient,
    middleware::ws_auth_middleware::WebSocketParams,
    services::jwt_service::validate_token,
    websocket::types::ChatMessage,
};

use super::{
    connection_manager::ConnectionManager,
    types::{StatusMessage, UserStatus, WebSocketMessage},
};

/// Handles the initial WebSocket upgrade request.
/// It extracts the chat_id from the query parameters and validates the user token.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    Extension(state): Extension<AppState>,
    Query(params): Query<WebSocketParams>,  // Here we extract the chat_id from the query parameters
) -> impl IntoResponse {
    // Check if the 'Authorization' header is present
    if let Some(authorization) = headers.get("Authorization") {
        // Convert the header value to a string
        if let Ok(token) = authorization.to_str() {
            // Validate the token and get the user_id
            if let Some(user_id) = validate_token(token) {
                // Upgrade the connection to a WebSocket and handle it
                return ws.on_upgrade(move |socket| {
                    handle_websocket_connection(socket, state, params.chat_id, user_id)
                });
            }
        }
    }

    // Return UNAUTHORIZED status if token validation fails
    StatusCode::UNAUTHORIZED.into_response()
}

/// Checks if the user is allowed to send messages in the specified chat.
/// It queries the database to confirm if the user is an accepted member or the chat creator.
async fn can_user_send_message(
    connection_manager: &ConnectionManager,
    db: DbClient,
    chat_id: Uuid,
    user_id: Uuid,
) -> Result<bool, String> {
    log::info!(
        "Checking if user with ID {} can send messages in chat with ID {}",
        user_id,
        chat_id
    );

    // SQL query to check if the user is an accepted member or the creator of the chat
    let query = "
        SELECT 1 FROM chat_members 
        WHERE chat_id = $1 AND user_id = $2 
        AND (status = 'accepted' OR is_creator = true)
    ";

    match db.query_opt(query, &[&chat_id, &user_id]).await {
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

/// Handles the WebSocket connection once it has been upgraded.
/// It validates the user's permission, sets up message broadcasting,
/// and listens for both incoming and outgoing messages.
async fn handle_websocket_connection(
    mut socket: WebSocket,
    state: AppState,
    chat_id: Uuid,
    user_id: Uuid,
) {
    let conn_manager = &state.connections;
    // Clone the DB client from state (assuming it's wrapped in an Arc)
    let db = state.db.clone();

    // Verify if the user is allowed to send messages in the chat
    if let Err(e) = can_user_send_message(conn_manager, db, chat_id, user_id).await {
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
        chat_id, // Pass the chat_id
        user_id,
        status: UserStatus::Online,
        timestamp: Utc::now().naive_utc(),
    });
    let _ = conn_manager.broadcast_message(status_msg, chat_id, user_id);

    // Main loop to handle incoming and outgoing WebSocket messages
    loop {
        tokio::select! {
            // Handle incoming messages from the WebSocket connection
            msg = socket.recv() => {
                match msg {
                    Some(Ok(msg)) => {
                        // Attempt to convert the message to text
                        if let Ok(text) = msg.to_text() {
                            // Create a new chat message with a unique message ID and the current timestamp
                            let chat_msg = WebSocketMessage::Chat(ChatMessage {
                                message_id: Uuid::new_v4(),
                                chat_id,
                                sender_id: user_id,
                                content: text.to_string(),
                                timestamp: Utc::now().naive_utc(),
                            });

                            // Broadcast the chat message to all users in the chat
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
                    None => break, // Connection closed
                }
            }
            // Handle messages received from the broadcast channel
            Ok(msg) = rx.recv() => {
                if let WebSocketMessage::Chat(chat_msg) = msg {
                    // Send the chat message to the WebSocket client
                    if let Err(e) = socket.send(Message::Text(chat_msg.content)).await {
                        eprintln!("Failed to send message: {}", e);
                        break;
                    }
                }
            }
        }
    }

    // Clean up when the connection is closed:
    // Update the user's status to Offline and remove the user from the in-memory chat.
    let _ = conn_manager.update_user_status(chat_id, user_id, UserStatus::Offline);
    let _ = conn_manager.remove_user_from_chat(chat_id, user_id);
}

/// Helper function to check if a user is already present in the in-memory representation of a chat.
pub async fn is_user_in_chat(
    connection_manager: &ConnectionManager,
    chat_id: Uuid,
    user_id: Uuid,
) -> Result<bool, String> {
    // Acquire a lock on the chats HashMap
    let chats = connection_manager.chats.lock().map_err(|_| "Lock error")?;

    // Check if the specified chat exists and if the user is present in that chat's users list
    Ok(chats
        .get(&chat_id)
        .map(|chat| chat.users.contains_key(&user_id))
        .unwrap_or(false))
}

/*
/// Helper function to retrieve the status of a user in a chat (currently commented out).
pub async fn get_user_status(
    connection_manager: &ConnectionManager,
    chat_id: Uuid,
    user_id: Uuid,
) -> Result<Option<UserStatus>, String> {
    let chats = connection_manager.chats.lock().map_err(|_| "Lock error")?;
    
    Ok(chats
        .get(&chat_id)
        .and_then(|chat| chat.users.get(&user_id))
        .map(|conn| conn.status.clone()))
}
*/
