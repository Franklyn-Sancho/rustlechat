use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;


#[derive(Serialize, Deserialize, Debug)]
pub struct Chat {
    pub id: Uuid,
    pub name: String,  
}

#[derive(Deserialize, Validate, Debug)]
pub struct CreateChatData {
    #[validate(length(min = 1, max = 50, message = "The chat must be between 1 and 59 characters long"))]
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateChatRequest {
    pub name: Option<String>,
}
