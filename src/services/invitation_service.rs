// services/invitation_service.rs

use crate::repositories::invitation_repository::InvitationRepository;
use crate::models::invitation::{ChatInvitation};
use uuid::Uuid;

pub struct InvitationService {
    repository: InvitationRepository,
}

impl InvitationService {
    pub fn new(repository: InvitationRepository) -> Self {
        InvitationService { repository }
    }

    pub async fn update_invitation_status(
        &self,
        invitation_id: Uuid,
        user_id: Uuid,
        accepted: bool,
    ) -> Result<ChatInvitation, String> {
        let status = if accepted { "accepted" } else { "rejected" };
        
        // Lógica de atualização de convite no repositório
        self.repository.update_invitation_status(invitation_id, user_id, accepted).await
    }

    pub async fn send_invitation(
        &self,
        chat_id: Uuid,
        inviter_id: Uuid,
        invitee_username: &str,
    ) -> Result<Uuid, String> {
        // Busca o ID do usuário baseado no nome
        let invitee_id = match self.repository.get_user_id_by_username(&invitee_username).await {
            Ok(Some(id)) => id,
            Ok(None) => return Err("User not found".to_string()),
            Err(e) => return Err(e),
        };

        // Envia o convite
        self.repository.send_invitation(chat_id, inviter_id, invitee_id).await
    }

    pub async fn add_user_to_chat(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), String> {
        self.repository.insert_user_to_chat(chat_id, user_id).await
    }
}



