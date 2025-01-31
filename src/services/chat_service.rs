use uuid::Uuid;

use crate::{
    database::init::DbClient,
    models::{
        chat::{Chat, CreateChatData},
        message::Message,
    }, services::invitation_service::create_invitation,
};

pub async fn create_chat(
    db: DbClient,
    user_id: Uuid,
    name: Option<String>,
) -> Result<Chat, String> {
    let chat_id = Uuid::new_v4();
    let chat_name = name.unwrap_or_else(|| "Default Chat".to_string());

    let query = "
        INSERT INTO chats (id, name) 
        VALUES ($1, $2)
        RETURNING id, name
    ";

    match db.query_one(query, &[&chat_id, &chat_name]).await {
        Ok(row) => {
            let insert_member_query = "
                INSERT INTO chat_members (chat_id, user_id) 
                VALUES ($1, $2)
            ";

            match db.execute(insert_member_query, &[&chat_id, &user_id]).await {
                Ok(_) => Ok(Chat {
                    id: row.get(0),
                    name: row.get(1),
                }),
                Err(e) => Err(format!("Failed to add member to chat: {:?}", e)),
            }
        }
        Err(e) => Err(format!("Failed to create chat: {:?}", e)),
    }
}

pub async fn get_chat_messages(db: DbClient, chat_id: Uuid) -> Result<Vec<Message>, String> {
    let query = "
        SELECT m.id, m.sender_id, m.message_text, m.timestamp 
        FROM messages m
        WHERE m.chat_id = $1
        ORDER BY m.timestamp
    ";

    let rows = db
        .query(query, &[&chat_id])
        .await
        .map_err(|e| format!("Erro ao buscar mensagens: {}", e))?;

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

pub async fn send_message(
    db: DbClient,
    chat_id: Uuid,
    sender_id: Uuid,
    message: String,
) -> Result<Message, String> {
    let message_id = Uuid::new_v4();

    let query = "
        INSERT INTO messages (id, chat_id, sender_id, message_text)
        VALUES ($1, $2, $3, $4)
        RETURNING id, chat_id, sender_id, message_text, timestamp
    ";

    let row = db
        .query_one(query, &[&message_id, &chat_id, &sender_id, &message])
        .await
        .map_err(|e| format!("Erro ao enviar mensagem: {}", e))?;

    Ok(Message {
        id: row.get(0),
        chat_id: row.get(1),
        sender_id: row.get(2),
        message_text: row.get(3),
        timestamp: row.get(4),
    })
}
