// services/chat_service.rs

use std::sync::Arc;
use uuid::Uuid;
use crate::{
    models::{chat::Chat, message::Message},
    repositories::chat_repository::ChatRepository,
};
use deadpool_postgres::{Pool, Transaction};

pub struct ChatService;

impl ChatService {
    /// Creates a new chat
    pub async fn create_chat(pool: Pool, user_id: Uuid, name: Option<String>) -> Result<Chat, String> {
        let chat_id = Uuid::new_v4();
        let chat_name = name.unwrap_or_else(|| "Default Chat".to_string());
        
        let mut client = pool.get().await.map_err(|e| format!("Failed to get DB client: {}", e))?;
        let transaction = client.transaction().await.map_err(|e| format!("Failed to start transaction: {}", e))?;
        
        // Create the chat and add the user as a member
        let (chat_id, chat_name) = ChatRepository::create_chat(&transaction, chat_id, &chat_name)
            .await.map_err(|e| format!("Failed to create chat: {}", e))?;
        
        // Add the user as a member of the chat
        ChatRepository::add_chat_member(&transaction, chat_id, user_id)
            .await.map_err(|e| format!("Failed to add creator to chat_members: {}", e))?;
        
        // Commit the transaction
        transaction.commit().await.map_err(|e| format!("Failed to commit transaction: {}", e))?;
        
        // Return the created chat
        Ok(Chat {
            id: chat_id,
            name: chat_name,
        })
    }

    /// Fetches all messages in a specific chat
    pub async fn get_chat_messages(pool: Pool, chat_id: Uuid) -> Result<Vec<Message>, String> {
        let mut client = pool.get().await.map_err(|e| format!("Failed to get DB client: {}", e))?;
        let transaction = client.transaction().await.map_err(|e| format!("Failed to start transaction: {}", e))?;

        // Fetch messages for the chat
        ChatRepository::get_chat_messages(&transaction, chat_id)
            .await.map_err(|e| format!("Error fetching messages: {}", e))
    }

    /// Sends a message in a chat
    pub async fn send_message(pool: Pool, chat_id: Uuid, sender_id: Uuid, message_text: String) -> Result<Message, String> {
        let message_id = Uuid::new_v4();
        let mut client = pool.get().await.map_err(|e| format!("Failed to get DB client: {}", e))?;
        let transaction = client.transaction().await.map_err(|e| format!("Failed to start transaction: {}", e))?;

        // Check if the sender is a member of the chat
        let is_member = ChatRepository::check_user_membership(&transaction, chat_id, sender_id)
            .await.map_err(|e| format!("Error checking chat membership: {}", e))?;
        
        if !is_member {
            return Err("User is not a member of this chat.".to_string());
        }

        // Insert the message
        ChatRepository::insert_message(&transaction, message_id, chat_id, sender_id, &message_text)
            .await.map_err(|e| format!("Error inserting message: {}", e))?;

        // Fetch the inserted message
        let message = ChatRepository::get_message_by_id(&transaction, message_id)
            .await.map_err(|e| format!("Error retrieving message: {}", e))?;

        // Commit the transaction
        transaction.commit().await.map_err(|e| format!("Failed to commit transaction: {}", e))?;
        
        Ok(message)
    }
}






