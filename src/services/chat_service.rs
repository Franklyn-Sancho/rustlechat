use std::sync::Arc;

use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde_json::json;
use tokio_postgres::Client;
use tracing::debug;
use uuid::Uuid;

use crate::{
    database::init::DbClient,
    models::{
        chat::{Chat, CreateChatData},
        message::Message,
    },
};

// Creates a new chat and adds the user to the chat as a member.
pub async fn create_chat(
    db: DbClient,
    user_id: Uuid,
    name: Option<String>,
) -> Result<Chat, String> {
    let chat_id = Uuid::new_v4();  // Generate a new unique chat ID
    let chat_name = name.unwrap_or_else(|| "Default Chat".to_string());  // Use the provided name or default to "Default Chat"

    // Log the chat creation attempt
    println!(
        "Trying to create chat: id = {}, name = {}, user_id = {}",
        chat_id, chat_name, user_id
    );

    let query = "
        INSERT INTO chats (id, name) 
        VALUES ($1, $2)
        RETURNING id, name
    ";

    // Try to insert the new chat into the database
    match db.query_one(query, &[&chat_id, &chat_name]).await {
        Ok(row) => {
            // If chat creation is successful, add the user as a member of the chat
            let insert_member_query = "
                INSERT INTO chat_members (chat_id, user_id) 
                VALUES ($1, $2)
            ";

            match db.execute(insert_member_query, &[&chat_id, &user_id]).await {
                Ok(_) => Ok(Chat {
                    id: row.get(0),
                    name: row.get(1),
                }),
                Err(e) => {
                    eprintln!("Error adding member to chat: {:?}", e);
                    Err(format!("Error adding member to chat: {:?}", e))
                }
            }
        }
        Err(e) => {
            eprintln!("Error executing query: {:?}", e);
            Err(format!("Error creating chat: {:?}", e))
        }
    }
}

// Retrieves all messages from a specific chat.
pub async fn get_chat_messages(db: DbClient, chat_id: Uuid) -> Result<Vec<Message>, String> {
    let query = "
        SELECT m.id, m.sender_id, m.message_text, m.timestamp 
        FROM messages m
        WHERE m.chat_id = $1
        ORDER BY m.timestamp
    ";

    // Query the database for messages in the specified chat
    let rows = db
        .query(query, &[&chat_id])
        .await
        .map_err(|e| format!("Error fetching messages: {}", e))?;

    // Map the database rows to a vector of Message structs
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

// Sends a new message in a specified chat.
pub async fn send_message(
    db: DbClient,
    chat_id: Uuid,
    sender_id: Uuid,
    message: String,
) -> Result<Message, String> {
    let message_id = Uuid::new_v4();  // Generate a new unique message ID

    let query = "
        INSERT INTO messages (id, chat_id, sender_id, message_text)
        VALUES ($1, $2, $3, $4)
        RETURNING id, chat_id, sender_id, message_text, timestamp
    ";

    // Insert the new message into the database and fetch the inserted row
    let row = db
        .query_one(query, &[&message_id, &chat_id, &sender_id, &message])
        .await
        .map_err(|e| format!("Error sending message: {}", e))?;

    // Return the inserted message as a Message struct
    Ok(Message {
        id: row.get(0),
        chat_id: row.get(1),
        sender_id: row.get(2),
        message_text: row.get(3),
        timestamp: row.get(4),
    })
}

