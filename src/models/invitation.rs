use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, NaiveDateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Rejected,
    Expired
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatInvitation {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub status: String,
    pub created_at: NaiveDateTime, // Alterado para NaiveDateTime
    pub updated_at: NaiveDateTime, // Alterado para NaiveDateTime
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SendInvitationRequest {
    pub chat_id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvitationNotification {
    pub invitation_id: Uuid,
    pub chat_id: Uuid,
    pub inviter_username: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationResponse {
    pub invitation_id: Uuid,
    pub accept: bool,
}

#[derive(Serialize, Deserialize)]
pub struct DeclineInviteRequest {
    pub chat_id: Uuid, // ID do chat que o usuário está recusando o convite
}

pub struct AcceptInviteRequest {
    pub chat_id: Uuid, // ID do chat que o usuário está aceitando o convite
}

