use uuid::Uuid;

use crate::{
    database::init::DbClient,
    models::{
        chat::Chat,
        message::Message,
    }, 
};

// Creates a new chat
pub async fn create_chat(
    db: DbClient, // Database client
    user_id: Uuid, // The ID of the user creating the chat
    name: Option<String>, // Optional chat name
) -> Result<Chat, String> {
    let chat_id = Uuid::new_v4(); // Generate a new chat ID
    let chat_name = name.unwrap_or_else(|| "Default Chat".to_string()); // Default to "Default Chat" if no name is provided

    // Query to insert a new chat into the database
    let query = "
        INSERT INTO chats (id, name) 
        VALUES ($1, $2)
        RETURNING id, name
    ";

    match db.query_one(query, &[&chat_id, &chat_name]).await {
        Ok(row) => {
            // Query to insert the creator into the chat_members table
            let insert_member_query = "
                INSERT INTO chat_members (chat_id, user_id, status, is_creator) 
                VALUES ($1, $2, 'accepted', TRUE)
            ";

            match db.execute(insert_member_query, &[&chat_id, &user_id]).await {
                Ok(_) => Ok(Chat {
                    id: row.get(0),
                    name: row.get(1),
                }),
                Err(e) => Err(format!("Failed to add creator to chat_members: {:?}", e)),
            }
        }
        Err(e) => Err(format!("Failed to create chat: {:?}", e)),
    }
}

// Fetches all messages in a specific chat
pub async fn get_chat_messages(db: DbClient, chat_id: Uuid) -> Result<Vec<Message>, String> {
    // Query to fetch messages for a chat
    let query = "
        SELECT m.id, m.sender_id, m.message_text, m.timestamp 
        FROM messages m
        WHERE m.chat_id = $1
        ORDER BY m.timestamp
    ";

    let rows = db
        .query(query, &[&chat_id])
        .await
        .map_err(|e| format!("Error fetching messages: {}", e))?;

    // Map the query result into a vector of Message objects
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

// Sends a message in a chat
pub async fn send_message(
    db: DbClient, // Database client
    chat_id: Uuid, // The ID of the chat
    sender_id: Uuid, // The ID of the user sending the message
    message: String, // The message text
) -> Result<Message, String> {
    // Check if the user is a member of the chat
    let is_member = db
        .query_opt(
            "SELECT 1 FROM chat_members WHERE chat_id = $1 AND user_id = $2",
            &[&chat_id, &sender_id],
        )
        .await
        .map_err(|e| format!("Error checking chat membership: {}", e))?
        .is_some();

    if !is_member {
        return Err("User is not a member of this chat.".to_string());
    }

    let message_id = Uuid::new_v4(); // Generate a new message ID

    // Query to insert the new message into the database
    let query = "
        INSERT INTO messages (id, chat_id, sender_id, message_text)
        VALUES ($1, $2, $3, $4)
        RETURNING id, chat_id, sender_id, message_text, timestamp
    ";

    let row = db
        .query_one(query, &[&message_id, &chat_id, &sender_id, &message])
        .await
        .map_err(|e| format!("Error sending message: {}", e))?;

    // Return the created message
    Ok(Message {
        id: row.get(0),
        chat_id: row.get(1),
        sender_id: row.get(2),
        message_text: row.get(3),
        timestamp: row.get(4),
    })
}

