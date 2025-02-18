use crate::{models::user::Claims, services::auth_service::verify_session};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::env;

use tracing::debug;

pub async fn auth_middleware<B>(mut req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
    // Extract the token from the request's Authorization header
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let token = token.trim(); // Remove extra spaces
                debug!("Received JWT token: {}", token);

                // Retrieve the secret key from environment variables
                let secret_key = env::var("JWT_SECRET_KEY").map_err(|_| {
                    eprintln!("Error: JWT_SECRET_KEY not found in .env file.");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                // Validate the token
                match decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(secret_key.as_bytes()),
                    &Validation::default(),
                ) {
                    Ok(token_data) => {
                        let user_id = token_data.claims.sub;
                        debug!("Valid JWT token. User ID: {}", user_id);
                        
                        // Insert the user_id into the request extensions for later use
                        req.extensions_mut().insert(user_id);
                        
                        return Ok(next.run(req).await);
                    }
                    Err(e) => {
                        eprintln!("Error decoding JWT token: {}", e);
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
            }
        }
    }

    // If no valid JWT token or session is found, return unauthorized status
    eprintln!("Error: No valid JWT token or session found.");
    Err(StatusCode::UNAUTHORIZED)
}



