use std::env;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::models::user::Claims;

// Generates a JWT (JSON Web Token) for the user, valid for one day.
pub fn create_jwt(user_id: Uuid) -> String {
    // Create the claims with user ID as subject and expiration time set to 1 day from now
    let claims = Claims {
        sub: user_id.to_string(),  // Subject (user ID)
        exp: (chrono::Utc::now() + chrono::Duration::days(1)).timestamp() as usize,  // Expiration time (1 day from now)
    };

    // Fetch the secret key from the environment variable
    let secret_key = env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY not found in .env file");

    // Create a default JWT header
    let header = Header::default();

    // Encode the JWT using the header, claims, and secret key
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )
    .unwrap()  // If encoding fails, panic (unwrap should be handled more gracefully in production)
    .trim_end()  // Remove trailing whitespace
    .to_string();  // Convert the result to a String
    
    token  // Return the generated JWT
}

// Validates the provided JWT token and returns the user ID if valid.
pub fn validate_token(token: &str) -> Option<Uuid> {
    // Fetch the secret key from the environment variable
    let secret_key = env::var("JWT_SECRET_KEY").ok()?;

    // Define the validation settings using HS256 algorithm
    let validation = Validation::new(Algorithm::HS256);

    // Decode the token and verify its claims
    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret_key.as_bytes()),
        &validation,
    ) {
        Ok(data) => Uuid::parse_str(&data.claims.sub).ok(),  // Return the user ID if decoding is successful
        Err(_) => None,  // If token is invalid or expired, return None
    }
}

