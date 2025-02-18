use std::sync::Arc;
use tokio_postgres::Client;
use uuid::Uuid;

pub struct AuthRepository {
    db: Arc<Client>, // Database client
}

impl AuthRepository {
    // Constructor to create a new AuthRepository instance
    pub fn new(db: Arc<Client>) -> Self {
        Self { db }
    }

    // Checks if the username already exists
    pub async fn username_exists(&self, username: &str) -> Result<bool, String> {
        let query = "SELECT COUNT(*) FROM users WHERE username = $1";
        let row = self
            .db
            .query_one(query, &[&username])
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        let count: i64 = row.get(0);
        Ok(count > 0)
    }

    // Inserts a new user into the database
    pub async fn insert_user(
        &self,
        username: &str,
        email: &str,
        hashed_password: &str,
    ) -> Result<(), String> {
        let query = "INSERT INTO users (username, email, password) VALUES ($1, $2, $3)";
        self.db
            .execute(query, &[&username, &email, &hashed_password])
            .await
            .map_err(|e| format!("Failed to insert user: {}", e))?;
        Ok(())
    }

    // Fetches the user by username
    pub async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<(Uuid, String)>, String> {
        let query = "SELECT id, password FROM users WHERE username = $1";
        let row = self
            .db
            .query_opt(query, &[&username])
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        Ok(row.map(|row| (row.get(0), row.get(1))))
    }

    // Inserts a session into the database
    pub async fn insert_session(
        &self,
        user_id: Uuid,
        token: &str,
        expires_at: chrono::NaiveDateTime,
    ) -> Result<(), String> {
        let query = "INSERT INTO sessions (user_id, token, expires_at) VALUES ($1, $2, $3)";
        self.db
            .execute(query, &[&user_id, &token, &expires_at])
            .await
            .map_err(|e| format!("Failed to insert session: {}", e))?;
        Ok(())
    }

    // Verifies if the session is valid
    pub async fn verify_session(&self, token: &str) -> Result<Option<Uuid>, String> {
        let query = "SELECT user_id FROM sessions WHERE token = $1 AND expires_at > NOW()";
        let row = self
            .db
            .query_opt(query, &[&token])
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        Ok(row.map(|row| row.get(0)))
    }

    // Fetches the username by user_id
    pub async fn get_username(&self, user_id: Uuid) -> Result<String, String> {
        let query = "SELECT username FROM users WHERE id = $1";
        let row = self
            .db
            .query_one(query, &[&user_id])
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        let username: String = row.get(0);
        Ok(username)
    }
}
