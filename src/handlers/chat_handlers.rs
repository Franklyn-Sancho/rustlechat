use crate::{
    app_state::AppState,
    handlers::invitation_handlers::send_invitation_helper,
    models::{
        chat::{Chat, CreateChatRequest},
        message::SendMessageRequest,
    },
    services::chat_service::{self, ChatService},
};
use axum::{debug_handler, extract::Path, response::IntoResponse, Extension, Json};
use hyper::StatusCode;
use uuid::Uuid;

pub async fn create_chat(
    Extension(state): Extension<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateChatRequest>,
) -> Result<Json<Chat>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&user_id).expect("User should be authenticated");

    // Pass the Arc<Pool> to the service layer
    let chat = match ChatService::create_chat(state.db.clone(), user_id, payload.name.clone()).await {
        Ok(chat) => chat,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    };

    // Invite users if provided
    if let Some(invitees) = payload.invitees {
        for invitee_username in invitees {
            if let Err((status, e)) = send_invitation_helper(
                &state.db,
                &state.connections,
                chat.id,
                user_id,
                invitee_username.clone(),
            )
            .await
            {
                // Optionally log errors or handle them
                eprintln!("Error sending invitation: {}", e);
            }
        }
    }

    Ok(Json(chat))
}

#[debug_handler]
pub async fn get_chat_messages(
    Extension(state): Extension<AppState>,
    Path(chat_id): axum::extract::Path<Uuid>, // Extracts `chat_id` from the URL
) -> impl IntoResponse {
    // Pass the Arc<Pool> to the service layer
    let messages = ChatService::get_chat_messages(state.db.clone(), chat_id).await;

    match messages {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

#[debug_handler]
pub async fn send_message_handler(
    Extension(state): Extension<AppState>,
    Json(payload): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let user_id = state.current_user_id.expect("User should be authenticated");

    // Pass the Arc<Pool> to the service layer
    match ChatService::send_message(state.db.clone(), payload.chat_id, user_id, payload.message)
        .await
    {
        Ok(message) => Ok(Json(message)),
        Err(e) => Err((StatusCode::FORBIDDEN, e)), // Alterado para FORBIDDEN se o usuário não for membro
    }
}

