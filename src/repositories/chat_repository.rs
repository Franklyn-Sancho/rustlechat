// repository/chat_repository.rs

use chrono::Utc;
use deadpool_postgres::Transaction;
use tokio_postgres::Error;
use uuid::Uuid;
use crate::models::message::Message;

pub struct ChatRepository;

impl ChatRepository {
    /// Inserts a new chat into the database
    pub async fn create_chat(transaction: &Transaction<'_>, chat_id: Uuid, name: &str) -> Result<(Uuid, String), Error> {
        let query = "
            INSERT INTO chats (id, name) 
            VALUES ($1, $2)
            RETURNING id, name
        ";
        let row = transaction.query_one(query, &[&chat_id, &name]).await?;
        Ok((row.get(0), row.get(1)))
    }

    /// Inserts a user as a chat member
    pub async fn add_chat_member(transaction: &Transaction<'_>, chat_id: Uuid, user_id: Uuid) -> Result<(), Error> {
        let query = "
            INSERT INTO chat_members (chat_id, user_id, status, is_creator) 
            VALUES ($1, $2, 'accepted', TRUE)
        ";
        transaction.execute(query, &[&chat_id, &user_id]).await?;
        Ok(())
    }

    /// Fetches all messages in a specific chat
    pub async fn get_chat_messages(transaction: &Transaction<'_>, chat_id: Uuid) -> Result<Vec<Message>, Error> {
        let query = "
            SELECT m.id, m.sender_id, m.message_text, m.timestamp 
            FROM messages m
            WHERE m.chat_id = $1
            ORDER BY m.timestamp
        ";
        let rows = transaction.query(query, &[&chat_id]).await?;

        let messages = rows
            .iter()
            .map(|row| Message {
                id: row.get(0),
                chat_id: row.get(1),
                sender_id: row.get(2),
                message_text: row.get(3),
                timestamp: row.get(4),
            })
            .collect();

        Ok(messages)
    }

    /// Inserts a new message into the database
    pub async fn insert_message(transaction: &Transaction<'_>, message_id: Uuid, chat_id: Uuid, sender_id: Uuid, message_text: &str) -> Result<(), Error> {
        let query = "
            INSERT INTO messages (id, chat_id, sender_id, message_text) 
            VALUES ($1, $2, $3, $4)
        ";
        transaction.execute(query, &[&message_id, &chat_id, &sender_id, &message_text]).await?;
        Ok(())
    }

    /// Retrieves a message by its ID
    pub async fn get_message_by_id(transaction: &Transaction<'_>, message_id: Uuid) -> Result<Message, Error> {
        let query = "
            SELECT id, chat_id, sender_id, message_text, timestamp
            FROM messages
            WHERE id = $1
        ";
        let row = transaction.query_one(query, &[&message_id]).await?;

        Ok(Message {
            id: row.get(0),
            chat_id: row.get(1),
            sender_id: row.get(2),
            message_text: row.get(3),
            timestamp: row.get(4),
        })
    }

    /// Checks if a user is a member of a chat
    pub async fn check_user_membership(transaction: &Transaction<'_>, chat_id: Uuid, user_id: Uuid) -> Result<bool, Error> {
        let query = "
            SELECT 1 FROM chat_members WHERE chat_id = $1 AND user_id = $2
        ";
        let result = transaction.query_opt(query, &[&chat_id, &user_id]).await?;
        Ok(result.is_some())
    }
}


