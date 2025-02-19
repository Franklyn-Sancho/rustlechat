use axum::{extract::Path, response::IntoResponse, Extension, Json};
use chrono::Utc;
use hyper::StatusCode;
use uuid::Uuid;
use deadpool_postgres::Pool;

use crate::{
    app_state::AppState,
    models::invitation::{InvitationNotification, InvitationResponse},
    services::{
        auth_service::get_username,
        invitation_service::update_invitation_status,
    },
    websocket::{
        connection_manager::ConnectionManager,
        types::{StatusMessage, UserStatus, WebSocketMessage},
    },
};

pub async fn respond_to_invitation(
    Extension(state): Extension<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<InvitationResponse>,
) -> impl IntoResponse {
    let user_id = Uuid::parse_str(&user_id).unwrap();
    let invitation_id = payload.invitation_id;

    // Update the invitation status in the `invites` table
    match update_invitation_status(&state.db, invitation_id, user_id, payload.accept).await {
        Ok(invitation) => {
            if payload.accept {
                // Insert the new member into the `chat_members` table
                let insert_query = "
                    INSERT INTO chat_members (chat_id, user_id, status, is_creator)
                    VALUES ($1, $2, 'accepted', false)
                ";

                let client = match state.db.get().await {
                    Ok(client) => client,
                    Err(e) => return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to get client from pool: {}", e),
                    )),
                };

                if let Err(e) = client
                    .execute(insert_query, &[&invitation.chat_id, &user_id])
                    .await
                {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to add user to chat_members: {}", e),
                    ));
                }

                // Add the user to the in-memory connection (for WebSocket)
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
    pool: &Pool,
    connections: &ConnectionManager,
    chat_id: Uuid,
    inviter_id: Uuid,
    invitee_username: String,
) -> Result<(), (StatusCode, String)> {
    let invitee_id = match get_user_id_by_username(pool, &invitee_username).await {
        Some(id) => id,
        None => {
            return Err((StatusCode::BAD_REQUEST, "User not found".to_string()));
        }
    };

    let existing_member = pool
        .get()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get client from pool: {}", e)))?
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

    if let Err(e) = pool
        .get()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get client from pool: {}", e)))?
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
            inviter_username: get_username(pool, inviter_id)
                .await
                .unwrap_or_default(),
            timestamp: Utc::now().naive_utc(),
        });

        if let Err(e) = connections.send_direct_message(user.id, notification).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error sending WebSocket notification: {}", e),
            ));
        }
    }

    Ok(())
}

pub async fn get_user_id_by_username(pool: &Pool, username: &str) -> Option<Uuid> {
    let query = "SELECT id FROM users WHERE username = $1";

    // Obtemos o client do pool, e lidamos com erros utilizando map_err para converter em Option
    match pool.get().await {
        Ok(client) => {
            // Agora o código pode continuar como esperado
            match client.query_opt(query, &[&username]).await {
                Ok(Some(row)) => Some(row.get(0)),
                Ok(None) => None,
                Err(_) => None, // Caso de erro no banco
            }
        },
        Err(_) => None, // Caso de erro ao obter o cliente do pool
    }
}



