use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
    Extension,
};
use chrono::Utc;
use hyper::StatusCode;
use uuid::Uuid;

use crate::{
    app_state::AppState, middleware::ws_auth_middleware::WebSocketParams, services::jwt_service::validate_token, websocket::types::ChatMessage
};

use super::{
    connection_manager::ConnectionManager,
    types::{StatusMessage, UserStatus, WebSocketMessage},
};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<AppState>,
    Query(params): Query<WebSocketParams>,
) -> impl IntoResponse {
    match params.token {
        Some(token) => {
            if let Some(user_id) = validate_token(&token) {
                return ws.on_upgrade(move |socket| {
                    handle_websocket_connection(socket, state, params.chat_id, user_id)
                });
            }
        }
        None => {
            eprintln!("Missing or invalid token");
        }
    }

    StatusCode::UNAUTHORIZED.into_response()
}

async fn handle_websocket_connection(
    mut socket: WebSocket,
    state: AppState,
    chat_id: Uuid,
    user_id: Uuid,
) {
    let conn_manager = &state.connections;

    // Adiciona usuário ao chat
    let mut rx = match conn_manager.add_user_to_chat(chat_id, user_id).await {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("Failed to add user to chat: {}", e);
            return;
        }
    };

    // Notifica outros usuários que este usuário está online
    let status_msg = WebSocketMessage::Status(StatusMessage {
        chat_id, // Passando o chat_id
        user_id,
        status: UserStatus::Online,
        timestamp: Utc::now().naive_utc(),
    });
    let _ = conn_manager.broadcast_message(status_msg, chat_id, user_id);

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

    // Cleanup ao desconectar
    let _ = conn_manager.update_user_status(chat_id, user_id, UserStatus::Offline);
    let _ = conn_manager.remove_user_from_chat(chat_id, user_id);
}

// Função auxiliar para verificar se um usuário está em um chat específico
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

// Função auxiliar para obter o status de um usuário em um chat
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
