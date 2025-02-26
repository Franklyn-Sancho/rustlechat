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
   /*  pub async fn send_encrypted_message(
        pool: Pool,
        chat_id: Uuid,
        sender_id: Uuid,
        message: String,
        chat_key: &ChatKey,
    ) -> Result<Message, String> {
        println!("INICIANDO ENVIO DE MENSAGEM");
        log::info!("Starting encrypted message transaction for chat {} from user {}", chat_id, sender_id);
        
        let mut client = pool.get().await
            .map_err(|e| {
                log::error!("Database connection error: {}", e);
                format!("Database connection error: {}", e)
            })?;
        
        // Log transaction start
        log::info!("Beginning database transaction");
        let transaction = client.transaction().await
            .map_err(|e| {
                log::error!("Transaction creation error: {}", e);
                format!("Transaction error: {}", e)
            })?;
        
        // Verify membership
        log::info!("Checking user membership for chat_id={}, user_id={}", chat_id, sender_id);
        let is_member = ChatRepository::check_user_membership(&transaction, chat_id, sender_id)
            .await
            .map_err(|e| {
                log::error!("Membership check failed: {}", e);
                format!("Membership check failed: {}", e)
            })?;
        
        if !is_member {
            log::warn!("User {} is not a member of chat {}", sender_id, chat_id);
            return Err("User is not a member of this chat".to_string());
        }
        
        // Initialize crypto
        log::info!("Initializing crypto with key length: {}", chat_key.0.len());
        let crypto = MessageCrypto::new(chat_key)
            .map_err(|e| {
                log::error!("Crypto initialization failed: {}", e);
                format!("Crypto initialization failed: {}", e)
            })?;
        
        log::info!("Encrypting message with key fingerprint: {}", crypto.key_fingerprint);
        
        // Encrypt message
        let encrypted = crypto.encrypt(&message)
            .map_err(|e| {
                log::error!("Message encryption failed: {}", e);
                format!("Message encryption failed: {}", e)
            })?;
        
        let message_id = Uuid::new_v4();
        log::info!(
            "Storing encrypted message: id={}, chat={}, sender={}, fingerprint={}",
            message_id, chat_id, sender_id, encrypted.key_fingerprint
        );
        
        // Store encrypted message with improved error handling
        let storage_result = ChatRepository::insert_encrypted_message(
            &transaction,
            message_id,
            chat_id,
            sender_id,
            &encrypted,
        ).await;
        
        match storage_result {
            Ok(_) => {
                log::info!("Message stored successfully with id={}", message_id);
            },
            Err(e) => {
                log::error!("Failed to store message: {}", e);
                // Rollback transaction explicitly on error
                if let Err(rollback_err) = transaction.rollback().await {
                    log::error!("Failed to rollback transaction: {}", rollback_err);
                } else {
                    log::info!("Transaction rolled back successfully");
                }
                return Err(format!("Failed to store encrypted message: {}", e));
            }
        }
        
        // Commit transaction with better error handling
        log::info!("Committing transaction");
        let commit_result = transaction.commit().await;
        if let Err(e) = commit_result {
            log::error!("Failed to commit transaction: {}", e);
            return Err(format!("Failed to commit transaction: {}", e));
        }
        
        log::info!("Transaction committed successfully");
        
        Ok(Message {
            id: message_id,
            chat_id,
            sender_id,
            message_text: message,
            timestamp: Utc::now().naive_utc(),
        })
    } */
    

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
        log::info!("Getting chat messages for chat={}, user={}", chat_id, user_id);
        
        let mut client = pool
            .get()
            .await
            .map_err(|e| {
                log::error!("Database connection error when getting messages: {}", e);
                format!("Database connection error: {}", e)
            })?;
        
        log::info!("Beginning transaction for retrieving messages");
        let transaction = client
            .transaction()
            .await
            .map_err(|e| {
                log::error!("Transaction error when getting messages: {}", e);
                format!("Transaction error: {}", e)
            })?;
        
        // Verify chat membership
        log::info!("Checking user membership for reading messages");
        let is_member = ChatRepository::check_user_membership(&transaction, chat_id, user_id)
            .await
            .map_err(|e| {
                log::error!("Membership check failed for reading: {}", e);
                format!("Membership check failed: {}", e)
            })?;
        
        if !is_member {
            log::warn!("User {} is not a member of chat {} for reading", user_id, chat_id);
            return Err("User is not a member of this chat".to_string());
        }
        
        // Get all keys for this user and chat
        log::info!("Retrieving all chat keys for user");
        let all_keys = ChatRepository::get_all_chat_keys(&transaction, chat_id, user_id)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve chat keys: {}", e);
                format!("Failed to retrieve chat keys: {}", e)
            })?;
        
        if all_keys.is_empty() {
            log::warn!("No chat keys found for user {} in chat {}", user_id, chat_id);
            return Err("No chat keys found for this user".to_string());
        }
        
        log::info!("Found {} chat keys for user", all_keys.len());
        
        // Create a map of key fingerprints to chat keys
        let mut key_map = std::collections::HashMap::new();
        for key_bytes in all_keys {
            let chat_key = ChatKey(key_bytes.clone());
            let key_fingerprint = format!("{:x}", md5::compute(&key_bytes));
            log::debug!("Added key with fingerprint: {}", key_fingerprint);
            key_map.insert(key_fingerprint, chat_key);
        }
        
        // Get encrypted messages
        log::info!("Retrieving encrypted messages for chat {}", chat_id);
        let encrypted_messages = ChatRepository::get_encrypted_messages(&transaction, chat_id)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve messages: {}", e);
                format!("Failed to retrieve messages: {}", e)
            })?;
        
        log::info!("Retrieved {} encrypted messages", encrypted_messages.len());
        
        // Decrypt messages
        let mut decrypted_messages = Vec::new();
        let mut decryption_failures = 0;
        
        // Use a referência para iterar sobre encrypted_messages 
        // em vez de consumir a coleção
        for (id, sender_id, encrypted) in &encrypted_messages {
            log::debug!("Attempting to decrypt message id={} with fingerprint={}", id, encrypted.key_fingerprint);
            
            // Use the correct key based on the message's key fingerprint
            if let Some(chat_key) = key_map.get(&encrypted.key_fingerprint) {
                let crypto = match MessageCrypto::new(chat_key) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Failed to initialize crypto for message {}: {}", id, e);
                        decryption_failures += 1;
                        continue; // Skip messages we can't decrypt
                    }
                };
                
                match crypto.decrypt(&encrypted) {
                    Ok(decrypted_text) => {
                        log::debug!("Successfully decrypted message id={}", id);
                        decrypted_messages.push(Message {
                            id: *id,  // Precisamos derreferenciar aqui
                            chat_id,
                            sender_id: *sender_id,  // Precisamos derreferenciar aqui
                            message_text: decrypted_text,
                            timestamp: Utc::now().naive_utc(),
                        });
                    }
                    Err(e) => {
                        log::warn!("Failed to decrypt message {}: {}", id, e);
                        decryption_failures += 1;
                        continue; // Skip messages we can't decrypt
                    }
                }
            } else {
                log::warn!("No key found for fingerprint: {}", encrypted.key_fingerprint);
                decryption_failures += 1;
            }
        }
        
        if decryption_failures > 0 {
            log::warn!("Failed to decrypt {} out of {} messages", 
                decryption_failures, encrypted_messages.len());
        }
        
        log::info!("Committing read transaction");
        transaction
            .commit()
            .await
            .map_err(|e| {
                log::error!("Failed to commit read transaction: {}", e);
                format!("Failed to commit transaction: {}", e)
            })?;
        
        log::info!("Successfully retrieved and decrypted {} messages", decrypted_messages.len());
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
