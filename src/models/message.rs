use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub chat_id: Uuid,  
    pub sender_id: Uuid, 
    pub message_text: String,
    pub timestamp: NaiveDateTime, 
}

#[derive(Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub chat_id: Uuid,
    pub message: String,
}
