// services/auth_service.rs

use axum::{http::StatusCode, response::IntoResponse, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use deadpool_postgres::Pool;
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;
use validator::Validate;

use crate::{
    models::user::{LoginData, RegisterData},
    repositories::auth_repository::AuthRepository,
    services::jwt_service::create_jwt, utils::password_validator::PasswordValidator,
};

pub async fn register_user(pool: Arc<Pool>, payload: RegisterData) -> impl IntoResponse {
    // Validate input data
    if let Err(errors) = payload.validate() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "errors": errors }))).into_response();
    }

    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Database connection error" })),
            ).into_response();
        }
    };

    // Check username existence
    match AuthRepository::check_username_exists(&client, &payload.username).await {
        Ok(true) => {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "Username already exists" })),
            ).into_response();
        }
        Ok(false) => (),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Database error" })),
            ).into_response();
        }
    }

    // Validate password
    if !PasswordValidator::validate(&payload.password) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Password must meet security requirements"
            })),
        ).into_response();
    }

    // Hash password and create user
    let hashed_password = match hash(&payload.password, DEFAULT_COST) {
        Ok(hashed) => hashed,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Password processing error" })),
            ).into_response();
        }
    };

    match AuthRepository::create_user(&client, &payload, &hashed_password).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({ "message": "User registered successfully" })),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to create user" })),
        ).into_response(),
    }
}

pub async fn get_username(pool: &Pool, user_id: Uuid) -> Option<String> {
    // Get a client from the pool
    let client = pool.get().await.unwrap();

    let query = "SELECT username FROM users WHERE id = $1";
    
    match client.query_opt(query, &[&user_id]).await {
        Ok(Some(row)) => Some(row.get(0)),
        Ok(None) => None,
        Err(e) => {
            eprintln!("Database error: {}", e);
            None
        }
    }
}


pub async fn login_user(pool: Arc<Pool>, payload: LoginData) -> impl IntoResponse {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Database connection error" })),
            ).into_response();
        }
    };

    // Get user credentials
    let (user_id, stored_password) = match AuthRepository::get_user_credentials(&client, &payload.username).await {
        Ok(Some(creds)) => creds,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid credentials" })),
            ).into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Authentication error" })),
            ).into_response();
        }
    };

    // Verify password
    if !verify(&payload.password, &stored_password).unwrap_or(false) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid credentials" })),
        ).into_response();
    }

    // Create JWT and session
    let token = create_jwt(user_id);
    match AuthRepository::create_session(&client, user_id, &token).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "token": token,
                "type": "Bearer"
            })),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to create session" })),
        ).into_response(),
    }
}

