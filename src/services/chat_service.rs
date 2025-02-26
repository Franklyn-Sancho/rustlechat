use crate::{
    crypto::{ChatKey, MessageCrypto},
    models::{chat::Chat, message::Message},
    repositories::chat_repository::ChatRepository,
};
use chrono::Utc;
use deadpool_postgres::{Pool, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub struct ChatService;

impl ChatService {
    /// Creates a new chat
    /// 
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `user_id` - ID of the creator
    /// * `name` - Optional chat name
    /// 
    /// # Returns
    /// * Chat object or error message
    pub async fn create_chat(
        pool: Pool,
        user_id: Uuid, 
        name: Option<String>,
    ) -> Result<Chat, String> {
        let chat_id = Uuid::new_v4();
        let chat_name = name.unwrap_or_else(|| "Default Chat".to_string());
    
        let mut client = pool
            .get()
            .await
            .map_err(|e| format!("Failed to get DB client: {}", e))?;
        
        let transaction = client
            .transaction()
            .await
            .map_err(|e| format!("Failed to start transaction: {}", e))?;
    
        // 1. Create the chat
        let (chat_id, chat_name) =
            ChatRepository::create_chat(&transaction, chat_id, &chat_name)
                .await
                .map_err(|e| format!("Failed to create chat: {}", e))?;
    
        // 2. Add the creator as chat member
        ChatRepository::add_chat_member(&transaction, chat_id, user_id)
            .await
            .map_err(|e| format!("Failed to add creator to chat: {}", e))?;
    
        // 3. Generate and store the chat key
        let chat_key = MessageCrypto::generate_chat_key();
        ChatRepository::store_chat_key(&transaction, chat_id, user_id, chat_key.0.clone())
            .await
            .map_err(|e| format!("Failed to store chat key: {}", e))?;
    
        transaction
            .commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    
        Ok(Chat { id: chat_id, name: chat_name })
    }
    
    /// Shares a chat key with a new member
    /// 
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `chat_id` - Chat ID
    /// * `from_user` - User who has the key
    /// * `to_user` - User to receive the key
    pub async fn share_chat_key_with_new_member(
        pool: &Pool,
        chat_id: Uuid,
        from_user: Uuid,
        to_user: Uuid,
    ) -> Result<(), String> {
        let mut client = pool.get().await.map_err(|e| format!("DB error: {}", e))?;
        let transaction = client.transaction().await.map_err(|e| e.to_string())?;

        // Get existing chat key
        let key = ChatRepository::get_chat_key(&transaction, chat_id, from_user)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Chat key not found")?;

        // Store key for new member
        ChatRepository::store_chat_key(&transaction, chat_id, to_user, key)
            .await
            .map_err(|e| format!("Failed to store shared key: {}", e))?;

        transaction.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Sends an encrypted message in a chat
    /// 
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `chat_id` - Chat ID
    /// * `sender_id` - Message sender ID
    /// * `message` - Plain text message
    /// * `chat_key` - Chat key for encryption
    /// 
    /// # Returns
    /// * Message object or error
    pub async fn send_encrypted_message(
        pool: Pool,
        chat_id: Uuid,
        sender_id: Uuid,
        message: String,
        chat_key: &ChatKey,
    ) -> Result<Message, String> {
        let mut client = pool
            .get()
            .await
            .map_err(|e| format!("Database connection error: {}", e))?;
        
        let transaction = client
            .transaction()
            .await
            .map_err(|e| format!("Transaction error: {}", e))?;
        
        // Verify user is a chat member
        let is_member = ChatRepository::check_user_membership(&transaction, chat_id, sender_id)
            .await
            .map_err(|e| format!("Membership check failed: {}", e))?;
            
        if !is_member {
            return Err("User is not a member of this chat".to_string());
        }
        
        // Create crypto instance with the chat key
        let crypto = MessageCrypto::new(chat_key)
            .map_err(|e| format!("Crypto initialization failed: {}", e))?;
        
        // Encrypt the message
        let encrypted = crypto
            .encrypt(&message)
            .map_err(|e| format!("Message encryption failed: {}", e))?;
        
        // Store the encrypted message
        let message_id = Uuid::new_v4();
        ChatRepository::insert_encrypted_message(
            &transaction,
            message_id,
            chat_id,
            sender_id,
            &encrypted,
        )
        .await
        .map_err(|e| format!("Failed to store encrypted message: {}", e))?;
        
        transaction
            .commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        
        // Create Message object with the original text
        let message = Message {
            id: message_id,
            chat_id,
            sender_id,
            message_text: message,
            timestamp: Utc::now().naive_utc(),
        };
        
        Ok(message)
    }

    /// Retrieves and decrypts chat messages for a user
    /// 
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `chat_id` - Chat ID
    /// * `user_id` - User ID requesting messages
    /// 
    /// # Returns
    /// * Vector of decrypted messages or error
    pub async fn get_chat_messages(
        pool: Pool,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Message>, String> {
        let mut client = pool
            .get()
            .await
            .map_err(|e| format!("Database connection error: {}", e))?;
    
        let transaction = client
            .transaction()
            .await
            .map_err(|e| format!("Transaction error: {}", e))?;
    
        // Verify chat membership
        let is_member = ChatRepository::check_user_membership(&transaction, chat_id, user_id)
            .await
            .map_err(|e| format!("Membership check failed: {}", e))?;
    
        if !is_member {
            return Err("User is not a member of this chat".to_string());
        }
    
        // Get all keys for this user and chat
        let all_keys = ChatRepository::get_all_chat_keys(&transaction, chat_id, user_id)
            .await
            .map_err(|e| format!("Failed to retrieve chat keys: {}", e))?;
    
        if all_keys.is_empty() {
            return Err("No chat keys found for this user".to_string());
        }
    
        // Create a map of key fingerprints to chat keys
        let mut key_map = std::collections::HashMap::new();
        for key_bytes in all_keys {
            let chat_key = ChatKey(key_bytes.clone());
            let key_fingerprint = format!("{:x}", md5::compute(&key_bytes));
            key_map.insert(key_fingerprint, chat_key);
        }
    
        // Get encrypted messages
        let encrypted_messages = ChatRepository::get_encrypted_messages(&transaction, chat_id)
            .await
            .map_err(|e| format!("Failed to retrieve messages: {}", e))?;
    
        // Decrypt messages
        let mut decrypted_messages = Vec::new();
        for (id, sender_id, encrypted) in encrypted_messages {
            // Use the correct key based on the message's key fingerprint
            if let Some(chat_key) = key_map.get(&encrypted.key_fingerprint) {
                let crypto = match MessageCrypto::new(chat_key) {
                    Ok(c) => c,
                    Err(e) => {
                        continue; // Skip messages we can't decrypt
                    }
                };
    
                match crypto.decrypt(&encrypted) {
                    Ok(decrypted_text) => {
                        decrypted_messages.push(Message {
                            id,
                            chat_id,
                            sender_id,
                            message_text: decrypted_text,
                            timestamp: Utc::now().naive_utc(),
                        });
                    }
                    Err(_) => {
                        continue; // Skip messages we can't decrypt
                    }
                }
            }
        }
    
        transaction
            .commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    
        Ok(decrypted_messages)
    }

    /// Gets a chat key for a specific user
    /// 
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `chat_id` - Chat ID
    /// * `user_id` - User ID
    /// 
    /// # Returns
    /// * Chat key or error
    pub async fn get_chat_key(
        pool: &Pool,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<ChatKey, String> {
        let mut client = pool
            .get()
            .await
            .map_err(|e| format!("Database connection error: {}", e))?;

        let transaction = client
            .transaction()
            .await
            .map_err(|e| format!("Transaction error: {}", e))?;

        let key_bytes = match ChatRepository::get_chat_key(&transaction, chat_id, user_id).await {
            Ok(Some(key)) => key,
            Ok(None) => return Err("Chat key not found".to_string()),
            Err(e) => return Err(format!("Failed to retrieve chat key: {}", e)),
        };

        transaction.commit().await.map_err(|e| e.to_string())?;
        Ok(ChatKey(key_bytes))
    }
}
