use crate::{
    app_state::AppState, crypto::{ChatKey, MessageCrypto}, handlers::invitation_handlers::send_invitation_helper, models::{
        chat::{Chat, CreateChatRequest},
        message::SendMessageRequest,
    }, repositories::chat_repository::ChatRepository, services::chat_service::{self, ChatService}
};
use axum::{debug_handler, extract::Path, response::IntoResponse, Extension, Json};
use hyper::StatusCode;
use uuid::Uuid;

/// HTTP handler for creating a new chat
pub async fn create_chat(
    Extension(state): Extension<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateChatRequest>,
) -> Result<Json<Chat>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&user_id).expect("User should be authenticated");
    
    // Create the chat
    let chat = match ChatService::create_chat(state.db.clone(), user_id, payload.name.clone()).await {
        Ok(chat) => chat,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    };
    
    // Invite users if provided
    if let Some(invitees) = payload.invitees {
        for invitee_username in invitees {
            if let Err(_) = send_invitation_helper(
                &state.db,
                &state.connections,
                chat.id,
                user_id,
                invitee_username,
            )
            .await {
                // Log error but continue with other invitations
            }
        }
    }
    
    Ok(Json(chat))
}

/// HTTP handler for getting chat messages
#[debug_handler]
pub async fn get_chat_messages(
    Extension(state): Extension<AppState>,
    Path(chat_id): Path<Uuid>,
) -> impl IntoResponse {
    // Get authenticated user ID
    let user_id = match state.current_user_id {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "User not authenticated".to_string())),
    };
    
    // Get messages from service
    let messages = ChatService::get_chat_messages(
        state.db.clone(),
        chat_id,
        user_id
    ).await;
    
    match messages {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// HTTP handler for sending messages
pub async fn send_message_handler(
    Extension(state): Extension<AppState>,
    Json(payload): Json<SendMessageRequest>,
) -> impl IntoResponse {
    // Get authenticated user ID
    let user_id = match state.current_user_id {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "User not authenticated".to_string())),
    };
    
    // Get the chat key
    let chat_key = match ChatService::get_chat_key(&state.db, payload.chat_id, user_id).await {
        Ok(key) => key,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    };
    
    // Send encrypted message
    let message = ChatService::send_encrypted_message(
        state.db.clone(),
        payload.chat_id,
        user_id,
        payload.message,
        &chat_key,
    ).await;
    
    match message {
        Ok(msg) => Ok(Json(msg)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}