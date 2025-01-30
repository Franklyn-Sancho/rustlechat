use uuid::Uuid;

use crate::{
    database::init::DbClient,
    models::{
        chat::{Chat, CreateChatData},
        message::Message,
    },
};

pub async fn create_chat(
    db: DbClient,
    user_id: Uuid,
    name: Option<String>,
    invitees: Option<Vec<String>>, // Lista de usernames dos convidados
) -> Result<Chat, String> {
    let chat_id = Uuid::new_v4();
    let chat_name = name.unwrap_or_else(|| "Default Chat".to_string());

    println!(
        "Tentando criar chat: id = {}, name = {}, user_id = {}, invitees = {:?}",
        chat_id, chat_name, user_id, invitees
    );

    let query = "
        INSERT INTO chats (id, name) 
        VALUES ($1, $2)
        RETURNING id, name
    ";

    match db.query_one(query, &[&chat_id, &chat_name]).await {
        Ok(row) => {
            // Inserir o criador como membro
            let insert_member_query = "
                INSERT INTO chat_members (chat_id, user_id) 
                VALUES ($1, $2)
            ";

            match db.execute(insert_member_query, &[&chat_id, &user_id]).await {
                Ok(_) => {
                    // Se houver convidados, adicionar à lista de membros
                    if let Some(invitees) = invitees {
                        let user_id_query = "SELECT id FROM users WHERE username = $1";
                        
                        for invitee_username in invitees {
                            // Buscar o user_id de cada convidado
                            let invitee_user_id: Uuid = match db.query_one(user_id_query, &[&invitee_username]).await {
                                Ok(row) => row.get(0),
                                Err(_) => return Err(format!("Usuário convidado '{}' não encontrado", invitee_username)),
                            };

                            // Adicionar convidado ao chat
                            let insert_invitee_query = "
                                INSERT INTO chat_members (chat_id, user_id) 
                                VALUES ($1, $2)
                            ";
                            if let Err(e) = db.execute(insert_invitee_query, &[&chat_id, &invitee_user_id]).await {
                                eprintln!("Erro ao adicionar convidado ao chat: {:?}", e);
                                return Err(format!("Erro ao adicionar convidado ao chat: {:?}", e));
                            }
                        }
                    }

                    Ok(Chat {
                        id: row.get(0),
                        name: row.get(1),
                    })
                },
                Err(e) => {
                    eprintln!("Erro ao adicionar membro ao chat: {:?}", e);
                    Err(format!("Erro ao adicionar membro ao chat: {:?}", e))
                }
            }
        }
        Err(e) => {
            eprintln!("Erro ao executar query: {:?}", e);
            Err(format!("Erro ao criar chat: {:?}", e))
        }
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
