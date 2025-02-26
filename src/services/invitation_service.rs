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

    pub async fn send_invitation(
        &self,
        chat_id: Uuid,
        inviter_id: Uuid, // Deve ser o ID do criador (ex.: frank)
        invitee_username: &str,
    ) -> Result<Uuid, String> {
        // Busca o ID do usuário convidado baseado no username
        let invitee_id = self
            .repository
            .get_user_id_by_username(invitee_username)
            .await?
            .ok_or_else(|| "User not found".to_string())?;

        log::debug!(
            "Enviando convite: chat_id={}, inviter_id={}, invitee_id={}",
            chat_id,
            inviter_id,
            invitee_id
        );

        // Cria o convite com os parâmetros na ordem correta
        self.repository
            .send_invitation(chat_id, inviter_id, invitee_id)
            .await
    }

    pub async fn update_invitation_status(
        &self,
        invitation_id: Uuid,
        user_id: Uuid, // Deve ser o ID do convidado (invitee) respondendo
        accepted: bool,
    ) -> Result<ChatInvitation, String> {
        self.repository
            .update_invitation_status(invitation_id, user_id, accepted)
            .await
    }

    pub async fn add_user_to_chat(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), String> {
        self.repository.insert_user_to_chat(chat_id, user_id).await
    }
}




