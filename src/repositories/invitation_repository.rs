// repositories/invitation_repository.rs

use chrono::{NaiveDateTime, Utc};
use deadpool_postgres::{Client, Pool};
use uuid::Uuid;
use crate::models::invitation::ChatInvitation;

pub struct InvitationRepository {
    pool: Pool,
}

impl InvitationRepository {
    pub fn new(pool: Pool) -> Self {
        InvitationRepository { pool }
    }

    pub async fn create_invitation(
        &self,
        chat_id: Uuid,
        inviter_id: Uuid,
        invitee_id: Uuid,
    ) -> Result<ChatInvitation, String> {
        let invitation_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        
        let query = "
            INSERT INTO invites (id, chat_id, inviter_id, invitee_id, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, chat_id, inviter_id, invitee_id, status, created_at, updated_at
        ";

        let client: Client = self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;

        let row = client
            .query_opt(query, &[&invitation_id, &chat_id, &inviter_id, &invitee_id, &"pending", &now, &now])
            .await
            .map_err(|e| format!("Failed to create invitation: {}", e))?;

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
            None => Err("Failed to insert invitation".to_string()),
        }
    }

    pub async fn check_invite_exists(
        &self,
        client: &Client,
        invitation_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, String> {
        let check_query = "SELECT id FROM invites WHERE id = $1 AND invitee_id = $2";
        let existing_invite = client
            .query_opt(check_query, &[&invitation_id, &user_id])
            .await
            .map_err(|e| format!("Failed to fetch invitation: {}", e))?;

        Ok(existing_invite.is_some())
    }

    pub async fn get_user_id_by_username(
        &self,
        username: &str,
    ) -> Result<Option<Uuid>, String> {
        let query = "SELECT id FROM users WHERE username = $1";

        let client: Client = self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;

        match client.query_opt(query, &[&username]).await {
            Ok(Some(row)) => Ok(Some(row.get(0))),
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to get user ID: {}", e)),
        }
    }

    pub async fn insert_user_to_chat(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), String> {
        let query = "
            INSERT INTO chat_members (chat_id, user_id, status, is_creator)
            VALUES ($1, $2, 'accepted', false)
        ";

        let client: Client = self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;

        client
            .execute(query, &[&chat_id, &user_id])
            .await
            .map_err(|e| format!("Failed to add user to chat_members: {}", e))?;

        Ok(())
    }

    pub async fn send_invitation(
        &self,
        chat_id: Uuid,
        inviter_id: Uuid,
        invitee_id: Uuid,
    ) -> Result<Uuid, String> {
        let invitation_id = Uuid::new_v4();
        let query = "
            INSERT INTO invites (id, chat_id, inviter_id, invitee_id, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
        ";

        let client: Client = self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;

        client
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
            .map_err(|e| format!("Failed to send invitation: {}", e))?;

        Ok(invitation_id)
    }

    pub async fn update_invitation_status(
        &self,
        invitation_id: Uuid,
        user_id: Uuid,
        accepted: bool,
    ) -> Result<ChatInvitation, String> {
        let status = if accepted { "accepted" } else { "rejected" };
        let now = Utc::now().naive_utc();

        let query = "
            UPDATE invites
            SET status = $1, updated_at = $2
            WHERE id = $3 AND invitee_id = $4
            RETURNING id, chat_id, inviter_id, invitee_id, status, created_at, updated_at
        ";

        let client: Client = self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;

        let row = client
            .query_opt(query, &[&status, &now, &invitation_id, &user_id])
            .await
            .map_err(|e| format!("Failed to update invitation status: {}", e))?;

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
            None => Err("Invitation not found or user is not the invitee".to_string()),
        }
    }
}

