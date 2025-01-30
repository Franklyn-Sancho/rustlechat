use axum::{
    extract::{Query, WebSocketUpgrade}, http::HeaderMap, middleware::Next, response::Response, Extension, TypedHeader
};
use hyper::{Request, StatusCode};
use serde::Deserialize;
use tracing::{info, error};
use uuid::Uuid;

use crate::{services::auth_service::verify_session, websocket::types::AppState};

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
    // Log the received query parameters and Authorization header
    info!("Query params: {:?}", params);
    info!("Authorization header: {:?}", headers.get("Authorization"));
    
    // Try to get the token from the query parameter or Authorization header
    let token = if let Some(token) = params.token {
        info!("Using token from query params");
        token
    } else if let Some(auth) = headers.get("Authorization") {
        info!("Using token from Authorization header");
        match auth.to_str() {
            Ok(auth_str) => {
                let token = auth_str.trim_start_matches("Bearer ").to_string(); // Extract token after "Bearer "
                info!("Extracted token: {}", token);
                token
            }
            Err(e) => {
                error!("Failed to parse Authorization header: {}", e);
                return Err(StatusCode::UNAUTHORIZED);  // Return unauthorized if header parsing fails
            }
        }
    } else {
        error!("No token found in query params or Authorization header");
        return Err(StatusCode::UNAUTHORIZED);  // Return unauthorized if no token is found
    };

    // Verify the token by calling the verify_session function
    match verify_session(state.db.clone(), &token).await {
        Ok(user_id) => {
            info!("Token verified successfully for user: {}", user_id);
            
            // Create a new state with the user_id from the verified token
            let mut new_state = state.clone();
            new_state.current_user_id = Some(user_id);
            
            // Insert the new state into the request extensions for downstream handlers
            let mut request = request;
            request.extensions_mut().insert(new_state);
            
            // Continue to the next handler
            Ok(next.run(request).await)
        }
        Err(e) => {
            error!("Token verification failed: {:?}", e);
            Err(StatusCode::UNAUTHORIZED)  // Return unauthorized if token verification fails
        }
    }
}