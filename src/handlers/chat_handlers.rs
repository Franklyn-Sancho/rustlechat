use axum::{debug_handler, extract::{Path, Query}, response::IntoResponse, Extension, Json};
use hyper::StatusCode;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;
use crate::{models::{chat::{CreateChatData, CreateChatRequest}, message::SendMessageRequest}, services::chat_service};

use super::websocker_handlers::AppState;


pub async fn create_chat(
    Extension(state): Extension<AppState>,
    Extension(user_id): Extension<String>, 
    Json(payload): Json<CreateChatRequest>,
) -> impl IntoResponse {
    let user_id = Uuid::parse_str(&user_id).expect("User should be authenticated"); // <- Converts to Uuid

    println!("Creating chat: user_id = {}, name = {:?}", user_id, payload.name);

    let chat = chat_service::create_chat(state.db.clone(), user_id, payload.name).await;

    match chat {
        Ok(chat) => {
            println!("Chat created successfully: {:?}", chat);
            Ok(Json(chat))
        },
        Err(e) => {
            eprintln!("Error creating chat: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        },
    }
}

#[debug_handler]
pub async fn get_chat_messages(
    Extension(state): Extension<AppState>,
    Path(chat_id): axum::extract::Path<Uuid> , // Extracts `chat_id` from the URL
) -> impl IntoResponse {
    let messages = chat_service::get_chat_messages(state.db.clone(), chat_id).await;

    match messages {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

#[debug_handler]
pub async fn send_message(
    Extension(state): Extension<AppState>,
    Json(payload): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let user_id = state.current_user_id.expect("User should be authenticated");
    let message = chat_service::send_message(state.db.clone(), payload.chat_id, user_id, payload.message).await;

    match message {
        Ok(message) => Ok(Json(message)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}