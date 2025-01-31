use std::sync::Arc;

use chrono::{NaiveDateTime, Utc};
use tokio_postgres::Client;
use uuid::Uuid;

use crate::{
    database::init::DbClient,
    handlers::invitation_handlers::get_user_id_by_username,
    models::invitation::{ChatInvitation, InvitationStatus},
};

impl std::fmt::Display for InvitationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InvitationStatus::Pending => "pending",
                InvitationStatus::Accepted => "accepted",
                InvitationStatus::Rejected => "rejected",
                InvitationStatus::Expired => "expired",
            }
        )
    }
}

pub async fn create_invitation(
    db: &DbClient,
    chat_id: Uuid,
    inviter_id: Uuid,
    invitee_id: Uuid,
) -> Result<ChatInvitation, String> {
    let now = Utc::now().naive_utc();

    let invitation = ChatInvitation {
        id: Uuid::new_v4(),
        chat_id,
        inviter_id,
        invitee_id,
        status: InvitationStatus::Pending.to_string(),
        created_at: now,
        updated_at: now,
    };

    db.execute(
        "INSERT INTO invites 
         (id, chat_id, inviter_id, invitee_id, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &[
            &invitation.id,
            &invitation.chat_id,
            &invitation.inviter_id,
            &invitation.invitee_id,
            &"pending",
            &invitation.created_at,
            &invitation.updated_at,
        ],
    )
    .await
    .map_err(|e| format!("Failed to create invitation: {}", e))?;

    Ok(invitation)
}

pub async fn update_invitation_status(
    db: &Arc<Client>,
    invitation_id: Uuid,
    user_id: Uuid,
    accepted: bool,
) -> Result<ChatInvitation, String> {
    let status = if accepted { "accepted" } else { "rejected" };
    let now = Utc::now().naive_utc();

    let check_query = "SELECT id FROM invites WHERE id = $1 AND invitee_id = $2";

    let existing_invite = db
        .query_opt(check_query, &[&invitation_id, &user_id])
        .await
        .map_err(|e| format!("Failed to fetch invitation: {}", e))?;

    if existing_invite.is_none() {
        return Err(format!("No invitation found for id: {} and user: {}", invitation_id, user_id));
    }

    let update_query = "
        UPDATE invites 
        SET status = $1, updated_at = $2
        WHERE id = $3 AND invitee_id = $4
        RETURNING id, chat_id, inviter_id, invitee_id, status, created_at, updated_at
    ";

    let row = db
        .query_opt(update_query, &[&status, &now, &invitation_id, &user_id])
        .await
        .map_err(|e| format!("Failed to update invitation: {}", e))?;

    match row {
        Some(row) => Ok(ChatInvitation {
            id: row.get(0),
            chat_id: row.get(1),
            inviter_id: row.get(2),
            invitee_id: row.get(3),
            status: row.get(4),
            created_at: row.get::<_, NaiveDateTime>(5),
            updated_at: row.get::<_, NaiveDateTime>(6),
        }),
        None => Err("No invitation was updated! It may have already been accepted or rejected.".to_string()),
    }
}
