use crate::{
    app_state::AppState, handlers::invitation_handlers::send_invitation_helper, models::{
        chat::{Chat, CreateChatData, CreateChatRequest}, invitation::SendInvitationRequest, message::SendMessageRequest
    }, services::chat_service
};
use axum::{
    debug_handler,
    extract::{Path, Query},
    response::IntoResponse,
    Extension, Json,
};
use hyper::StatusCode;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

pub async fn create_chat(
    Extension(state): Extension<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateChatRequest>,
) -> Result<Json<Chat>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&user_id).expect("User should be authenticated");

    let chat = match chat_service::create_chat(
        state.db.clone(),
        user_id,
        payload.name.clone(),
    )
    .await
    {
        Ok(chat) => chat,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    };

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
    let message =
        chat_service::send_message(state.db.clone(), payload.chat_id, user_id, payload.message)
            .await;

    match message {
        Ok(message) => Ok(Json(message)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
