use chrono::{DateTime, Utc};
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
        let now: DateTime<Utc> = Utc::now();
    
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
                created_at: row.get::<_, DateTime<Utc>>(5),
                updated_at: row.get::<_, DateTime<Utc>>(6),
            }),
            None => Err("Failed to insert invitation".to_string()),
        }
    }

    pub async fn update_invitation_status(
        &self,
        invitation_id: Uuid,
        user_id: Uuid,
        accepted: bool,
    ) -> Result<ChatInvitation, String> {
        let status = if accepted { "accepted" } else { "rejected" };
        let now: DateTime<Utc> = Utc::now();
    
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
                created_at: row.get::<_, DateTime<Utc>>(5),
                updated_at: row.get::<_, DateTime<Utc>>(6),
            }),
            None => Err("Invitation not found or user is not the invitee".to_string()),
        }
    }

    pub async fn insert_user_to_chat(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), String> {
        // First check if the user is already a member of this chat
        let check_query = "
            SELECT 1 FROM chat_members 
            WHERE chat_id = $1 AND user_id = $2
        ";
    
        let client: Client = self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;
    
        let exists = client
            .query_opt(check_query, &[&chat_id, &user_id])
            .await
            .map_err(|e| format!("Failed to check membership: {}", e))?
            .is_some();
    
        // If the user is already a member, just return success
        if exists {
            return Ok(());
        }
    
        // Otherwise, proceed with the insert
        let query = "
            INSERT INTO chat_members (chat_id, user_id, status, is_creator)
            VALUES ($1, $2, 'accepted', false)
        ";
    
        client
            .execute(query, &[&chat_id, &user_id])
            .await
            .map_err(|e| format!("Failed to add user to chat_members: {}", e))?;
    
        Ok(())
    }

    pub async fn get_user_id_by_username(
        &self,
        username: &str,
    ) -> Result<Option<Uuid>, String> {
        let query = "
            SELECT id FROM users WHERE username = $1
        ";

        let client: Client = self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;

        let row = client
            .query_opt(query, &[&username])
            .await
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        match row {
            Some(row) => Ok(Some(row.get(0))),
            None => Ok(None), // Caso o usuário não seja encontrado
        }
    }


    pub async fn send_invitation(
        &self,
        chat_id: Uuid,
        inviter_id: Uuid,
        invitee_id: Uuid,
    ) -> Result<Uuid, String> {
        let invitation_id = Uuid::new_v4();
        let now: DateTime<Utc> = Utc::now();

        let query = "
            INSERT INTO invites (id, chat_id, inviter_id, invitee_id, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
        ";

        let client: Client = self
            .pool
            .get()
            .await
            .map_err(|e| format!("Failed to get client from pool: {}", e))?;

        log::debug!(
            "Inserindo convite no BD: chat_id={}, inviter_id={}, invitee_id={}",
            chat_id,
            inviter_id,
            invitee_id
        );

        client
            .execute(
                query,
                &[
                    &invitation_id,
                    &chat_id,
                    &inviter_id, // ID do criador
                    &invitee_id, // ID do convidado
                    &"pending",
                    &now,
                    &now,
                ],
            )
            .await
            .map_err(|e| {
                log::error!("Falha ao inserir convite: {}", e);
                format!("Failed to send invitation: {}", e)
            })?;

        log::info!("Convite inserido com sucesso: {}", invitation_id);
        Ok(invitation_id)
    }

}


