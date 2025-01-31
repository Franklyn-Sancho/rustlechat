use axum::{http::StatusCode, response::IntoResponse, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use regex::Regex;
use serde_json::json;
use std::sync::Arc;
use tokio_postgres::Client;
use uuid::Uuid;
use validator::Validate;

use crate::{database::init::DbClient, models::user::{Claims, LoginData, RegisterData}};

use super::jwt_service::create_jwt;

// Function to check if the password meets strength requirements
fn is_password_strong(password: &str) -> bool {
    // Checks if the password contains at least one uppercase letter
    let has_uppercase = Regex::new(r"[A-Z]").unwrap().is_match(password);
    // Checks if the password contains at least one lowercase letter
    let has_lowercase = Regex::new(r"[a-z]").unwrap().is_match(password);
    // Checks if the password contains at least one digit
    let has_digit = Regex::new(r"\d").unwrap().is_match(password);
    // Checks if the password contains at least one special character
    let has_special_char = Regex::new(r"[@$!%*?&]").unwrap().is_match(password);
    // Checks if the password has a minimum length of 8 characters
    let has_minimum_length = password.len() >= 8;

    // Returns true if all conditions are met, otherwise false
    has_uppercase && has_lowercase && has_digit && has_special_char && has_minimum_length
}

// Function to register a new user
pub async fn register_user(db: Arc<Client>, payload: RegisterData) -> impl IntoResponse {
    // Validation of input data has already been done in the handler, no need to repeat here.

    let username = &payload.username;
    let email = &payload.email;
    let password = &payload.password;
    // Hash the password before storing it
    let hashed_password = hash(password, DEFAULT_COST).unwrap();

    // Check if the username already exists in the database
    let check_query = "SELECT COUNT(*) FROM users WHERE username = $1";
    let count: i64 = db
        .query_one(check_query, &[&username])
        .await
        .map(|row| row.get(0))
        .unwrap_or(0);

    // If the username already exists, return a conflict response
    if count > 0 {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "username already exists" })),
        )
            .into_response();
    }

    // Check if the password is strong
    if !is_password_strong(&payload.password) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "The password must contain uppercase letters, lowercase letters, numbers, and special characters" }))).into_response();
    }

    // Validate the input data (this is done after password strength validation)
    if let Err(errors) = payload.validate() {
        return (StatusCode::BAD_REQUEST, Json(errors)).into_response();
    }

    // Insert the new user into the database
    let insert_query = "INSERT INTO users (username, email, password) VALUES ($1, $2, $3)";
    match db
        .execute(insert_query, &[username, email, &hashed_password])
        .await
    {
        // If user registration is successful, return a successful response
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({ "message": "User registered successfully" })),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Failed to insert new user: {}", e);
            // If an error occurs during insertion, return an internal server error response
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to register user" })),
            )
                .into_response()
        }
    }
}

// Function to log in a user
pub async fn login_user(db: Arc<Client>, payload: LoginData) -> impl IntoResponse {
    let username = &payload.username;
    let password = &payload.password;

    let query = "SELECT id, password FROM users WHERE username = $1";

    match db.query_opt(query, &[&username]).await {
        // If the user is found in the database
        Ok(Some(row)) => {
            let stored_password: String = row.get(1);
            let user_id: Uuid = row.get(0);

            // Verify the password by comparing it with the stored hashed password
            if verify(password, &stored_password).unwrap_or(false) {
                // Generate a JWT token for the user
                let token = create_jwt(user_id);

                // Optionally, store the token or session information in the database
                let expires_at = (Utc::now() + chrono::Duration::days(30)).naive_utc();
                let insert_session_query =
                    "INSERT INTO sessions (user_id, token, expires_at) VALUES ($1, $2, $3)";
                if let Err(e) = db
                    .execute(insert_session_query, &[&user_id, &token, &expires_at])
                    .await
                {
                    eprintln!("Failed to create session: {}", e);
                    // Return an internal server error if session creation fails
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to create session",
                    )
                        .into_response();
                }

                // Return the generated JWT token as the response
                (StatusCode::OK, Json(json!({"bearer": token.trim()}))).into_response()
            } else {
                // If the password is incorrect, return an unauthorized response
                (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response()
            }
        }
        // If the user is not found, return a not found response
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            // If there is a database error, return an internal server error response
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

// Function to verify if the session token is valid
pub async fn verify_session(db: Arc<Client>, token: &str) -> Result<Uuid, StatusCode> {
    let query = "SELECT user_id FROM sessions WHERE token = $1 AND expires_at > NOW()";
    match db.query_opt(query, &[&token]).await {
        // If the session is found and is not expired
        Ok(Some(row)) => {
            let user_id: Uuid = row.get(0);
            Ok(user_id)
        }
        // If no session is found or the session has expired
        Ok(None) => Err(StatusCode::UNAUTHORIZED),
        Err(e) => {
            eprintln!("Database error: {}", e);
            // If there is a database error, return an internal server error response
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_username(db: &DbClient, user_id: Uuid) -> Result<String, String> {
    // Query para buscar o username
    let query = "SELECT username FROM users WHERE id = $1";
    
    // Execute a query usando o cliente do postgres
    let row = db
        .query_one(query, &[&user_id])
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Extraia o username da row
    let username: String = row
        .try_get(0)
        .map_err(|e| format!("Failed to get username: {}", e))?;

    Ok(username)
}

