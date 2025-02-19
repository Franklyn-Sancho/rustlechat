// repository/auth_repository.rs

use chrono::Utc;
use deadpool_postgres::Client;
use uuid::Uuid;
use crate::models::user::RegisterData;

pub struct AuthRepository;

impl AuthRepository {
    /// Checks if a username already exists in the database
    pub async fn check_username_exists(client: &Client, username: &str) -> Result<bool, tokio_postgres::Error> {
        let query = "SELECT COUNT(*) FROM users WHERE username = $1";
        let count: i64 = client
            .query_one(query, &[&username])
            .await?
            .get(0);
        Ok(count > 0)
    }

    /// Creates a new user in the database
    pub async fn create_user(
        client: &Client, 
        user_data: &RegisterData, 
        hashed_password: &str
    ) -> Result<(), tokio_postgres::Error> {
        let query = "INSERT INTO users (username, email, password) VALUES ($1, $2, $3)";
        client
            .execute(query, &[&user_data.username, &user_data.email, &hashed_password])
            .await?;
        Ok(())
    }

    /// Gets user credentials for authentication
    pub async fn get_user_credentials(
        client: &Client,
        username: &str,
    ) -> Result<Option<(Uuid, String)>, tokio_postgres::Error> {
        let query = "SELECT id, password FROM users WHERE username = $1";
        let row = client.query_opt(query, &[&username]).await?;
        
        Ok(row.map(|row| {
            let user_id: Uuid = row.get(0);
            let password: String = row.get(1);
            (user_id, password)
        }))
    }

    /// Creates a new session for authenticated user
    pub async fn create_session(
        client: &Client,
        user_id: Uuid,
        token: &str,
    ) -> Result<(), tokio_postgres::Error> {
        let expires_at = (Utc::now() + chrono::Duration::days(30)).naive_utc();
        let query = "INSERT INTO sessions (user_id, token, expires_at) VALUES ($1, $2, $3)";
        
        client
            .execute(query, &[&user_id, &token, &expires_at])
            .await?;
        Ok(())
    }

    /// Verifies if a session token is valid
    pub async fn verify_session_token(
        client: &Client,
        token: &str,
    ) -> Result<Option<Uuid>, tokio_postgres::Error> {
        let query = "SELECT user_id FROM sessions WHERE token = $1 AND expires_at > NOW()";
        let row = client.query_opt(query, &[&token]).await?;
        Ok(row.map(|row| row.get(0)))
    }
}