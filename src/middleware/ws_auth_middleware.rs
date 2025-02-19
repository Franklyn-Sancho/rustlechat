use axum::{
    extract::{Query, WebSocketUpgrade}, http::HeaderMap, middleware::Next, response::Response, Extension, TypedHeader
};
use hyper::{Request, StatusCode};
use serde::Deserialize;
use tracing::{info, error};
use uuid::Uuid;

use crate::{app_state::AppState, repositories::auth_repository::AuthRepository};

#[derive(Deserialize, Debug)]
pub struct WebSocketParams {
    pub token: Option<String>,  // Optional token from query parameters
    pub chat_id: Uuid,          // The chat ID associated with the request
}

pub async fn ws_auth_middleware<B>(
    Query(params): Query<WebSocketParams>,
    headers: HeaderMap,
    Extension(state): Extension<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, String)> {
    info!("WebSocket connection attempt - Chat ID: {}", params.chat_id);

    // Extract token from query params or Authorization header
    let token = extract_token(&params, &headers)
        .map_err(|e| {
            error!("Token extraction failed: {}", e);
            (StatusCode::UNAUTHORIZED, e.to_string())
        })?;

    // Get client from connection pool
    let client = state.db.get().await.map_err(|e| {
        error!("Database connection failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to establish database connection".to_string(),
        )
    })?;

    // Verify session and get user ID
    let user_id = AuthRepository::verify_session_token(&client, &token)
        .await
        .map_err(|e| {
            error!("Session verification failed: {}", e);
            (StatusCode::UNAUTHORIZED, "Invalid or expired session".to_string())
        })?
        .ok_or_else(|| {
            error!("No session found for token");
            (StatusCode::UNAUTHORIZED, "Session not found".to_string())
        })?;

    info!("Session verified for user: {}", user_id);

    // Create new state with user ID
    let mut new_state = state.clone();
    new_state.current_user_id = Some(user_id);

    // Add chat access verification here if needed
    verify_chat_access(&client, user_id, params.chat_id).await
        .map_err(|e| {
            error!("Chat access verification failed: {}", e);
            (StatusCode::FORBIDDEN, "Access to chat denied".to_string())
        })?;

    // Update request extensions with new state
    let mut request = request;
    request.extensions_mut().insert(new_state);

    // Continue with the request
    Ok(next.run(request).await)
}

// Helper function to extract token from request
fn extract_token(params: &WebSocketParams, headers: &HeaderMap) -> Result<String, &'static str> {
    match (params.token.as_ref(), headers.get("Authorization")) {
        (Some(token), _) => {
            info!("Using token from query params");
            Ok(token.clone())
        }
        (None, Some(auth_header)) => {
            info!("Using token from Authorization header");
            auth_header
                .to_str()
                .map(|h| h.trim_start_matches("Bearer ").to_string())
                .map_err(|_| "Invalid Authorization header")
        }
        (None, None) => {
            error!("No authentication token provided");
            Err("No authentication token provided")
        }
    }
}

// Helper function to verify chat access
async fn verify_chat_access(
    client: &deadpool_postgres::Client,
    user_id: uuid::Uuid,
    chat_id: uuid::Uuid,  // Change this to Uuid
) -> Result<(), AuthError> {
    // Query adjusted for Uuid chat_id
    let query = "SELECT EXISTS(
        SELECT 1 FROM chat_members 
        WHERE chat_id = $1 AND user_id = $2
    )";
    
    let has_access: bool = client
        .query_one(query, &[&chat_id, &user_id])
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?
        .get(0);

    if !has_access {
        return Err(AuthError::AccessDenied(format!(
            "User {} does not have access to chat {}",
            user_id, chat_id
        )));
    }

    Ok(())
}


// Add this to your error.rs file
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Invalid token")]
    InvalidToken,
    
    #[error("Session expired")]
    SessionExpired,
}