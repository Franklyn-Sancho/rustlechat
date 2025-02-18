use axum::{
    extract::{Query, WebSocketUpgrade}, http::HeaderMap, middleware::Next, response::Response, Extension, TypedHeader
};
use hyper::{Request, StatusCode};
use serde::Deserialize;
use tracing::{info, error};
use uuid::Uuid;

use crate::{app_state::AppState, services::auth_service::verify_session};

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
) -> Result<Response, StatusCode> {
    info!("WebSocket connection attempt with params: {:?}", params);
    
    // Ensure chat_id is provided
    let chat_id = params.chat_id;
    info!("Chat ID received: {}", chat_id);

    // Get token from query params or header
    let token = match (params.token, headers.get("Authorization")) {
        (Some(token), _) => {
            info!("Using token from query params");
            token
        }
        (None, Some(auth_header)) => {
            info!("Using token from Authorization header");
            auth_header
                .to_str()
                .map_err(|e| {
                    error!("Failed to parse Authorization header: {}", e);
                    StatusCode::UNAUTHORIZED
                })?
                .trim_start_matches("Bearer ")
                .to_string()
        }
        (None, None) => {
            error!("No token provided in query params or Authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    info!("Attempting to verify token: {}", token);

    // Verify the session
    match verify_session(state.db.clone(), &token).await {
        Ok(user_id) => {
            info!("Session verified for user: {}", user_id);
            
            let mut new_state = state.clone();
            new_state.current_user_id = Some(user_id);
            
            let mut request = request;
            request.extensions_mut().insert(new_state);
            
            Ok(next.run(request).await)
        }
        Err(e) => {
            error!("Session verification failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}