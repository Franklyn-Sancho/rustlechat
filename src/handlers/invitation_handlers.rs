use std::sync::Arc;

use axum::{extract::Path, response::IntoResponse, Extension, Json};
use chrono::Utc;
use deadpool_postgres::Pool;
use hyper::StatusCode;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    models::invitation::{InvitationNotification, InvitationResponse},
    repositories::invitation_repository::InvitationRepository,
    services::{
        auth_service::AuthService, chat_service::ChatService, invitation_service::InvitationService,
    },
    websocket::{
        connection_manager::ConnectionManager,
        types::{StatusMessage, UserStatus, WebSocketMessage},
    },
};

/// Handler for responding to an invitation (accepting or rejecting)
pub async fn respond_to_invitation(
    Extension(state): Extension<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<InvitationResponse>,
) -> impl IntoResponse {
    // Converte o user_id autenticado (deve ser o do convidado que responde)
    let user_id = Uuid::parse_str(&user_id).unwrap();
    let invitation_id = payload.invitation_id;

    let invitation_service = InvitationService::new(InvitationRepository::new(state.db.clone()));

    match invitation_service
        .update_invitation_status(invitation_id, user_id, payload.accept)
        .await
    {
        Ok(invitation) => {
            if payload.accept {
                // Adiciona o convidado (user_id) ao chat somente após aceitar o convite
                if let Err(e) = invitation_service
                    .add_user_to_chat(invitation.chat_id, user_id)
                    .await
                {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
                }

                // Compartilha a chave do chat com o novo membro.
                // Note que usamos o invitation.inviter_id para buscar a chave original do criador.
                if let Err(e) = ChatService::share_chat_key_with_new_member(
                    &state.db,
                    invitation.chat_id,
                    invitation.inviter_id, // Deve ser o ID do criador (ex.: frank)
                    user_id,               // ID do convidado (ex.: lurrinha)
                )
                .await
                {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
                }

                // Adiciona o usuário à conexão WebSocket e envia notificação (se aplicável)
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

/// Helper function to send an invitation to a user
pub async fn send_invitation_helper(
    pool: &Pool,
    connections: &ConnectionManager,
    chat_id: Uuid,
    inviter_id: Uuid,
    invitee_username: String,
) -> Result<(), (StatusCode, String)> {
    let invitation_service = InvitationService::new(InvitationRepository::new(pool.clone()));

    log::info!("Sending invitation to username: {}", invitee_username);

    match invitation_service
        .send_invitation(chat_id, inviter_id, &invitee_username)
        .await
    {
        Ok(invitation_id) => {
            let auth_service = AuthService::new(pool.clone());

            log::info!("Invitation created successfully: {}", invitation_id);

            if let Some(user) = connections.get_online_user(&invitee_username).await {
                let notification = WebSocketMessage::Invitation(InvitationNotification {
                    invitation_id,
                    chat_id,
                    inviter_username: auth_service
                        .get_username(inviter_id)
                        .await
                        .unwrap_or_default(),
                    timestamp: Utc::now().naive_utc(),
                });

                log::info!("Sending WebSocket notification to user: {}", user.id);

                if let Err(e) = connections.send_direct_message(user.id, notification).await {
                    log::error!("Error sending WebSocket notification: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error sending WebSocket notification: {}", e),
                    ));
                }
            } else {
                log::warn!("User {} is not online", invitee_username);
            }

            Ok(())
        }
        Err(e) => {
            log::error!("Failed to send invitation: {}", e);
            Err((StatusCode::BAD_REQUEST, e))
        }
    }
}
