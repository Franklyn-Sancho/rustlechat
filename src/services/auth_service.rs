use axum::{http::StatusCode, response::IntoResponse, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use deadpool_postgres::Pool;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::{models::user::{LoginData, RegisterData}, repositories::auth_repository::AuthRepository, services::jwt_service::create_jwt, utils::password_validator::PasswordValidator};


pub struct AuthService {
    pool: Pool,
}

impl AuthService {
    pub fn new(pool: Pool) -> Self {
        AuthService { pool }
    }

    pub async fn register_user(&self, payload: RegisterData) -> impl IntoResponse {
        log::info!("Attempting to register user: {}", payload.username);

        // Validate input data
        if let Err(errors) = payload.validate() {
            log::warn!("Validation errors for register: {:?}", errors);
            return (StatusCode::BAD_REQUEST, Json(json!({ "errors": errors }))).into_response();
        }

        let client = match self.pool.get().await {
            Ok(client) => client,
            Err(_) => {
                log::error!("Failed to get database client");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Database connection error" })),
                )
                    .into_response();
            }
        };

        // Check username existence
        match AuthRepository::check_username_exists(&client, &payload.username).await {
            Ok(true) => {
                log::warn!("Username already exists: {}", payload.username);
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "Username already exists" })),
                )
                    .into_response();
            }
            Ok(false) => (),
            Err(_) => {
                log::error!("Database error while checking username");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Database error" })),
                )
                    .into_response();
            }
        }

        // Validate password
        if !PasswordValidator::validate(&payload.password) {
            log::warn!("Password validation failed for username: {}", payload.username);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Password must meet security requirements"
                })),
            )
                .into_response();
        }

        // Hash password and create user
        let hashed_password = match hash(&payload.password, DEFAULT_COST) {
            Ok(hashed) => hashed,
            Err(_) => {
                log::error!("Error hashing password for username: {}", payload.username);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Password processing error" })),
                )
                    .into_response();
            }
        };

        match AuthRepository::create_user(&client, &payload, &hashed_password).await {
            Ok(_) => {
                log::info!("User registered successfully: {}", payload.username);
                (
                    StatusCode::CREATED,
                    Json(json!({ "message": "User registered successfully" })),
                )
                    .into_response()
            }
            Err(_) => {
                log::error!("Failed to create user: {}", payload.username);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Failed to create user" })),
                )
                    .into_response()
            }
        }
    }

    pub async fn login_user(&self, payload: LoginData) -> impl IntoResponse {
        log::info!("Attempting login for username: {}", payload.username);

        let client = match self.pool.get().await {
            Ok(client) => client,
            Err(_) => {
                log::error!("Database connection error");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Database connection error" })),
                )
                    .into_response();
            }
        };

        // Get user credentials
        let (user_id, stored_password) =
            match AuthRepository::get_user_credentials(&client, &payload.username).await {
                Ok(Some(creds)) => creds,
                Ok(None) => {
                    log::warn!("Invalid credentials for username: {}", payload.username);
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({ "error": "Invalid credentials" })),
                    )
                        .into_response();
                }
                Err(_) => {
                    log::error!("Error fetching user credentials for username: {}", payload.username);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Authentication error" })),
                    )
                        .into_response();
                }
            };

        // Verify password
        if !verify(&payload.password, &stored_password).unwrap_or(false) {
            log::warn!("Password verification failed for username: {}", payload.username);
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid credentials" })),
            )
                .into_response();
        }

        // Create JWT and session
        let token = create_jwt(user_id);
        log::info!("JWT generated for user_id: {}", user_id);

        match AuthRepository::create_session(&client, user_id, &token).await {
            Ok(_) => {
                log::info!("Session created successfully for user_id: {}", user_id);
                (
                    StatusCode::OK,
                    Json(json!({
                        "token": token,
                        "type": "Bearer"
                    })),
                )
                    .into_response()
            }
            Err(_) => {
                log::error!("Failed to create session for user_id: {}", user_id);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Failed to create session" })),
                )
                    .into_response()
            }
        }
    }

    pub async fn get_username(&self, user_id: Uuid) -> Option<String> {
        log::info!("Fetching username for user_id: {}", user_id);

        let client = match self.pool.get().await {
            Ok(client) => client,
            Err(_) => {
                log::error!("Failed to get database client");
                return None;
            }
        };

        let query = "SELECT username FROM users WHERE id = $1";

        match client.query_opt(query, &[&user_id]).await {
            Ok(Some(row)) => {
                let username: String = row.get(0);
                log::info!("Found username: {}", username);
                Some(username)
            }
            Ok(None) => {
                log::warn!("No username found for user_id: {}", user_id);
                None
            }
            Err(e) => {
                log::error!("Error fetching username for user_id: {}: {}", user_id, e);
                None
            }
        }
    }
}

