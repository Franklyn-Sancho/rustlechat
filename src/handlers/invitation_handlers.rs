use axum::{extract::Path, response::IntoResponse, Extension, Json};
use chrono::Utc;
use hyper::StatusCode;
use uuid::Uuid;

use crate::{
    app_state::AppState, database::init::DbClient, models::invitation::{
        AcceptInviteRequest, DeclineInviteRequest, InvitationNotification, InvitationResponse,
        SendInvitationRequest,
    }, services::{
        auth_service::get_username,
        invitation_service::{self, create_invitation, update_invitation_status},
    }, websocket::{
        connection_manager::ConnectionManager,
        types::{StatusMessage, UserStatus, WebSocketMessage},
    }
};

pub async fn respond_to_invitation(
    Extension(state): Extension<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<InvitationResponse>,
) -> impl IntoResponse {
    let user_id = Uuid::parse_str(&user_id).unwrap();
    let invitation_id = payload.invitation_id;

    match update_invitation_status(&state.db, invitation_id, user_id, payload.accept).await {
        Ok(invitation) => {
            if payload.accept {
                if let Ok(_) = state
                    .connections
                    .add_user_to_chat(invitation.chat_id, user_id)
                    .await
                {
                    let notification = WebSocketMessage::Status(StatusMessage {
                        chat_id: invitation.chat_id,
                        user_id,
                        status: UserStatus::Joined,
                        timestamp: Utc::now().naive_utc(),
                    });

                    let _ = state
                        .connections
                        .broadcast_to_chat(invitation.chat_id, user_id, notification)
                        .await;
                }
            }
            Ok(Json(invitation))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

pub async fn send_invitation_helper(
    db: &DbClient,
    connections: &ConnectionManager,
    chat_id: Uuid,
    inviter_id: Uuid,
    invitee_username: String,
) -> Result<(), (StatusCode, String)> {
    let invitee_id = match get_user_id_by_username(db, &invitee_username).await {
        Some(id) => id,
        None => {
            return Err((StatusCode::BAD_REQUEST, "User not found".to_string()));
        }
    };

    let existing_member = db
        .query_opt(
            "SELECT user_id FROM chat_members WHERE chat_id = $1 AND user_id = $2",
            &[&chat_id, &invitee_id],
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if existing_member.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "User is already a member of this chat".to_string(),
        ));
    }

    let invitation_id = Uuid::new_v4();
    let query = "
    INSERT INTO invites (id, chat_id, inviter_id, invitee_id, status, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7)
";

    if let Err(e) = db
        .execute(
            query,
            &[
                &invitation_id,
                &chat_id,
                &inviter_id,
                &invitee_id,
                &"pending",
                &Utc::now().naive_utc(),
                &Utc::now().naive_utc(),
            ],
        )
        .await
    {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create invitation: {}", e)));
    }

    if let Some(user) = connections.get_online_user(&invitee_username).await {
        let notification = WebSocketMessage::Invitation(InvitationNotification {
            invitation_id,
            chat_id,
            inviter_username: get_username(db, inviter_id)
                .await
                .unwrap_or_default(),
            timestamp: Utc::now().naive_utc(),
        });

        if let Err(e) = connections.send_direct_message(user.id, notification).await {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Error sending WebSocket notification: {}", e)));
        }
    }

    Ok(())
}

pub async fn get_user_id_by_username(db: &DbClient, username: &str) -> Option<Uuid> {
    let query = "SELECT id FROM users WHERE username = $1";

    match db.query_one(query, &[&username]).await {
        Ok(row) => Some(row.get(0)),
        Err(_) => None,
    }
}

