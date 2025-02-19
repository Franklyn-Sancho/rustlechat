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
    services::{auth_service::AuthService, invitation_service::InvitationService},
    websocket::{
        connection_manager::ConnectionManager,
        types::{StatusMessage, UserStatus, WebSocketMessage},
    },
};

/// Handler for responding to an invitation (accepting or rejecting)
pub async fn respond_to_invitation(
    Extension(state): Extension<AppState>, // Extracts the application state
    Extension(user_id): Extension<String>, // Extracts the user ID from the request
    Json(payload): Json<InvitationResponse>, // Extracts the invitation response payload
) -> impl IntoResponse {
    // Parse the user ID string into a UUID
    let user_id = Uuid::parse_str(&user_id).unwrap();
    let invitation_id = payload.invitation_id;

    // Create an instance of InvitationService with the database pool
    let invitation_service = InvitationService::new(InvitationRepository::new(state.db.clone()));

    // Update the invitation status (accept or reject)
    match invitation_service
        .update_invitation_status(invitation_id, user_id, payload.accept)
        .await
    {
        Ok(invitation) => {
            // If the invitation is accepted, add the user to the chat
            if payload.accept {
                if let Err(e) = invitation_service
                    .add_user_to_chat(invitation.chat_id, user_id)
                    .await
                {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
                }

                // Add the user to the WebSocket connection
                if let Ok(_) = state
                    .connections
                    .add_user_to_chat(invitation.chat_id, user_id)
                    .await
                {
                    // Create a WebSocket notification for the user joining the chat
                    let notification = WebSocketMessage::Status(StatusMessage {
                        chat_id: invitation.chat_id,
                        user_id,
                        status: UserStatus::Joined,
                        timestamp: Utc::now().naive_utc(),
                    });
                    // Broadcast the notification to the chat
                    let _ = state
                        .connections
                        .broadcast_to_chat(invitation.chat_id, user_id, notification)
                        .await;
                }
            }
            // Return the updated invitation as a JSON response
            Ok(Json(invitation))
        }
        // Return an error if updating the invitation status fails
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

/// Helper function to send an invitation to a user
pub async fn send_invitation_helper(
    pool: &Pool, // Database connection pool
    connections: &ConnectionManager, // WebSocket connection manager
    chat_id: Uuid, // ID of the chat
    inviter_id: Uuid, // ID of the user sending the invitation
    invitee_username: String, // Username of the user being invited
) -> Result<(), (StatusCode, String)> {
    // Create an instance of InvitationService with the database pool
    let invitation_service = InvitationService::new(InvitationRepository::new(pool.clone()));

    // Send the invitation (passing a reference to `invitee_username`)
    match invitation_service
        .send_invitation(chat_id, inviter_id, &invitee_username)
        .await
    {
        Ok(invitation_id) => {
            // Create an instance of AuthService to fetch the inviter's username
            let auth_service = AuthService::new(pool.clone());

            // Check if the invitee is online
            if let Some(user) = connections.get_online_user(&invitee_username).await {
                // Create a WebSocket notification for the invitation
                let notification = WebSocketMessage::Invitation(InvitationNotification {
                    invitation_id,
                    chat_id,
                    inviter_username: auth_service
                        .get_username(inviter_id)
                        .await
                        .unwrap_or_default(), // Fetch the inviter's username
                    timestamp: Utc::now().naive_utc(),
                });

                // Send the notification directly to the invitee
                if let Err(e) = connections.send_direct_message(user.id, notification).await {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error sending WebSocket notification: {}", e),
                    ));
                }
            }

            // Return success if everything works
            Ok(())
        }
        // Return an error if sending the invitation fails
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}