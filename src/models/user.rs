use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;


#[derive(Deserialize, Validate)]
pub struct RegisterData {
    #[validate(length(min = 3, max = 20, message = "The username must be between 3 and 20 characters long"))]
    pub username: String,
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(min = 6, message = "Password must be at least 6 characters long"))]
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  
    pub exp: usize, 
}