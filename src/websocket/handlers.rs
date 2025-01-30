use axum::{
    extract::{ws::{Message, WebSocket}, Query, WebSocketUpgrade},
    response::{IntoResponse, Response}, Extension,
};
use hyper::StatusCode;
use uuid::Uuid;

use crate::{middleware::ws_auth_middleware::WebSocketParams, services::jwt_service::validate_token, websocket::types::ChatMessage};

use super::types::{AppState, StatusMessage, UserStatus, WebSocketMessage};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<AppState>,
    Query(params): Query<WebSocketParams>,
) -> impl IntoResponse {
    match params.token {
        Some(token) => {
            if let Some(user_id) = validate_token(&token) {
                return ws.on_upgrade(move |socket| handle_websocket_connection(socket, state, params.chat_id, user_id));
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
    let mut rx = match conn_manager.add_user_to_chat(chat_id, user_id) {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("Failed to add user to chat: {}", e);
            return;
        }
    };

    // Notifica outros usuários que este usuário está online
    let status_msg = WebSocketMessage::Status(StatusMessage {
        user_id,
        status: UserStatus::Online,
        timestamp: chrono::Utc::now(),
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
                                timestamp: chrono::Utc::now(),
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