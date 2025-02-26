use crate::{crypto::EncryptedMessage, models::message::Message};
use chrono::Utc;
use deadpool_postgres::Transaction;
use tokio_postgres::Error;
use uuid::Uuid;

pub struct ChatRepository;

impl ChatRepository {
    /// Creates a new chat in the database
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `chat_id` - UUID for the new chat
    /// * `name` - Name of the chat
    ///
    /// # Returns
    /// * Tuple of chat_id and name on success
    pub async fn create_chat(
        transaction: &Transaction<'_>,
        chat_id: Uuid,
        name: &str,
    ) -> Result<(Uuid, String), Error> {
        let query = "
            INSERT INTO chats (id, name) 
            VALUES ($1, $2)
            RETURNING id, name
        ";
        let row = transaction.query_one(query, &[&chat_id, &name]).await?;
        Ok((row.get(0), row.get(1)))
    }

    /// Adds a user as a chat member
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `chat_id` - Chat ID
    /// * `user_id` - User ID to add
    pub async fn add_chat_member(
        transaction: &Transaction<'_>,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Error> {
        let query = "
            INSERT INTO chat_members (chat_id, user_id, status, is_creator) 
            VALUES ($1, $2, 'accepted', TRUE)
        ";
        transaction.execute(query, &[&chat_id, &user_id]).await?;
        Ok(())
    }

    /// Checks if a user is a member of a chat
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `chat_id` - Chat ID
    /// * `user_id` - User ID to check
    ///
    /// # Returns
    /// * Boolean indicating membership status
    pub async fn check_user_membership(
        transaction: &Transaction<'_>,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, Error> {
        let query = "
            SELECT 1 FROM chat_members WHERE chat_id = $1 AND user_id = $2
        ";
        let result = transaction.query_opt(query, &[&chat_id, &user_id]).await?;
        Ok(result.is_some())
    }

    /// Inserts an encrypted message into the database
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `message_id` - Message UUID
    /// * `chat_id` - Chat ID
    /// * `sender_id` - Sender's user ID
    /// * `encrypted` - Encrypted message data
    pub async fn insert_encrypted_message(
        transaction: &Transaction<'_>,
        message_id: Uuid,
        chat_id: Uuid,
        sender_id: Uuid,
        encrypted: &EncryptedMessage,
    ) -> Result<(), String> {
        let query = r#"
            INSERT INTO encrypted_messages
            (id, chat_id, sender_id, encrypted_content, nonce, key_fingerprint, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP)
        "#;

        transaction
            .execute(
                query,
                &[
                    &message_id,
                    &chat_id,
                    &sender_id,
                    &encrypted.encrypted_content,
                    &encrypted.nonce,
                    &encrypted.key_fingerprint,
                ],
            )
            .await
            .map_err(|e| format!("Failed to insert message: {}", e))?;

        Ok(())
    }

    /// Retrieves all encrypted messages for a chat
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `chat_id` - Chat ID
    ///
    /// # Returns
    /// * Vector of message ID, sender ID, and encrypted message content
    pub async fn get_encrypted_messages(
        transaction: &Transaction<'_>,
        chat_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, EncryptedMessage)>, String> {
        let query = "SELECT id, sender_id, encrypted_content, nonce, key_fingerprint 
                     FROM encrypted_messages 
                     WHERE chat_id = $1 
                     ORDER BY created_at";

        let rows = transaction
            .query(query, &[&chat_id])
            .await
            .map_err(|e| e.to_string())?;

        let messages = rows
            .iter()
            .map(|row| {
                let id: Uuid = row.get(0);
                let sender_id: Uuid = row.get(1);
                let encrypted_content: Vec<u8> = row.get(2);
                let nonce: Vec<u8> = row.get(3);
                let key_fingerprint: String = row.get(4);

                (
                    id,
                    sender_id,
                    EncryptedMessage {
                        encrypted_content,
                        nonce,
                        key_fingerprint,
                    },
                )
            })
            .collect();

        Ok(messages)
    }

    /// Gets all chat keys for a user in a specific chat
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `chat_id` - Chat ID
    /// * `user_id` - User ID
    ///
    /// # Returns
    /// * Vector of key bytes
    pub async fn get_all_chat_keys(
        transaction: &Transaction<'_>,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Vec<u8>>, String> {
        let query = "SELECT encrypted_key FROM chat_keys WHERE chat_id = $1 AND user_id = $2";

        let rows = transaction
            .query(query, &[&chat_id, &user_id])
            .await
            .map_err(|e| e.to_string())?;

        let keys = rows.iter().map(|row| row.get::<_, Vec<u8>>(0)).collect();

        Ok(keys)
    }

    /// Stores a chat key for a user
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `chat_id` - Chat ID
    /// * `user_id` - User ID
    /// * `encrypted_key` - Encrypted key data
    pub async fn store_chat_key(
        transaction: &Transaction<'_>,
        chat_id: Uuid,
        user_id: Uuid,
        encrypted_key: Vec<u8>,
    ) -> Result<(), String> {
        let query = "
            INSERT INTO chat_keys (chat_id, user_id, encrypted_key)
            VALUES ($1, $2, $3)
        ";
        transaction
            .execute(query, &[&chat_id, &user_id, &encrypted_key])
            .await
            .map_err(|e| format!("Failed to store chat key: {}", e))?;

        Ok(())
    }

    /// Retrieves a chat key for a user
    ///
    /// # Arguments
    /// * `transaction` - Database transaction
    /// * `chat_id` - Chat ID
    /// * `user_id` - User ID
    ///
    /// # Returns
    /// * Optional vector of key bytes
    /// Gets a chat key for a user
    pub async fn get_chat_key(
        transaction: &Transaction<'_>,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Vec<u8>>, String> {
        let query = "
            SELECT encrypted_key FROM chat_keys
            WHERE chat_id = $1 AND user_id = $2
        ";
        let row = transaction
            .query_opt(query, &[&chat_id, &user_id])
            .await
            .map_err(|e| format!("Failed to get chat key: {}", e))?;

        Ok(row.map(|r| r.get(0)))
    }
}
