use uuid::Uuid;
use deadpool_postgres::Pool;
use crate::{
    crypto::{ChatKey, MessageCrypto},
    repositories::chat_repository::ChatRepository,
};

pub struct KeyExchangeService;

impl KeyExchangeService {
    /// Initializes chat keys for a new chat
    /// 
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `chat_id` - Chat ID
    /// * `creator_id` - Creator's user ID
    /// 
    /// # Returns
    /// * Chat key or error
    pub async fn initialize_chat_keys(
        pool: &Pool,
        chat_id: Uuid,
        creator_id: Uuid,
    ) -> Result<ChatKey, String> {
        let mut client = pool.get().await.map_err(|e| e.to_string())?;
        let transaction = client.transaction().await.map_err(|e| e.to_string())?;
        
        // Generate new chat key
        let chat_key = MessageCrypto::generate_chat_key();
        
        // Store key for creator
        ChatRepository::store_chat_key(&transaction, chat_id, creator_id, chat_key.0.clone())
            .await
            .map_err(|e| e.to_string())?;
            
        transaction.commit().await.map_err(|e| e.to_string())?;
        Ok(chat_key)
    }
    
    /// Shares a chat key with another user
    /// 
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `chat_id` - Chat ID
    /// * `from_user` - User sharing the key
    /// * `to_user` - User receiving the key
    pub async fn share_chat_key(
        pool: &Pool,
        chat_id: Uuid,
        from_user: Uuid,
        to_user: Uuid,
    ) -> Result<(), String> {
        let mut client = pool.get().await.map_err(|e| e.to_string())?;
        let transaction = client.transaction().await.map_err(|e| e.to_string())?;
        
        // Get chat key
        let encrypted_key = ChatRepository::get_chat_key(&transaction, chat_id, from_user)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Chat key not found")?;
        
        // Store for new user
        ChatRepository::store_chat_key(&transaction, chat_id, to_user, encrypted_key)
            .await
            .map_err(|e| e.to_string())?;
            
        transaction.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}