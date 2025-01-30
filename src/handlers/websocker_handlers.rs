use std::{
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

use crate::{database::init::DbClient, middleware::ws_auth_middleware::WebSocketParams, services::jwt_service::validate_token};

type Connections = Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>;

#[derive(Clone)]
pub struct AppState {
    pub connections: Connections,  // Manages WebSocket connections
    pub db: Arc<Client>,  // Database connection
    pub current_user_id: Option<Uuid>,  // Currently authenticated user's ID
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
    let (tx, mut rx) = broadcast::channel(32);

    {
        let mut chat_conns = state.connections.lock().unwrap();
        chat_conns.insert(chat_id, tx.clone()); // Inserts the `Sender` directly
    }

    println!("User {} joined chat {}", user_id, chat_id);

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(msg)) => {
                        if let Ok(text) = msg.to_text() {
                            println!("{} sent: {}", user_id, text);

                            if let Err(e) = save_message(&state.db, chat_id, text.to_string(), user_id).await {
                                eprintln!("Error saving message: {}", e);
                            }

                            // Sends the message to all users in the same chat
                            let chat_conns = state.connections.lock().unwrap();
                            if let Some(conn) = chat_conns.get(&chat_id) {
                                let _ = conn.send(text.to_string());
                            }
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("Error receiving message: {}", e);
                        break;
                    }
                    None => break,
                }
            }

            Ok(msg) = rx.recv() => {
                if let Err(e) = socket.send(Message::Text(msg)).await {
                    eprintln!("Error sending message: {}", e);
                    break;
                }
            }
        }
    }

    // Removes the connection from the HashMap when exiting
    let mut chat_conns = state.connections.lock().unwrap();
    chat_conns.remove(&chat_id);

    println!("User {} left chat {}", user_id, chat_id);
}

async fn save_message(
    db: &DbClient,
    chat_id: Uuid,
    message: String,
    user_id: Uuid,
) -> Result<(), String> {
    let query = "
        INSERT INTO messages (chat_id, sender_id, message_text)
        VALUES ($1, $2, $3)
    ";

    db.execute(query, &[&chat_id, &user_id, &message])
        .await
        .map_err(|e| format!("Error saving message: {}", e))?;

    Ok(())
}
