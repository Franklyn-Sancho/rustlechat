use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use uuid::Uuid;

use crate::models::user::RegisterData;

pub struct AuthRepository;

impl AuthRepository {
    // Função para verificar se o nome de usuário já existe
    pub async fn check_username_exists(client: &Client, username: &str) -> Result<bool, tokio_postgres::Error> {
        let query = "SELECT COUNT(*) FROM users WHERE username = $1";
        log::info!("Checking if username exists: {}", username);

        let count: i64 = match client.query_one(query, &[&username]).await {
            Ok(row) => row.get(0),
            Err(e) => {
                log::error!("Error checking username existence: {}", e);
                return Err(e);
            }
        };

        log::info!("Username exists: {}", count > 0);
        Ok(count > 0)
    }

    // Função para criar um novo usuário
    pub async fn create_user(
        client: &Client, 
        user_data: &RegisterData, 
        hashed_password: &str
    ) -> Result<(), tokio_postgres::Error> {
        let query = "INSERT INTO users (username, email, password) VALUES ($1, $2, $3)";
        log::info!("Creating user with username: {}", user_data.username);

        match client.execute(query, &[&user_data.username, &user_data.email, &hashed_password]).await {
            Ok(_) => {
                log::info!("User created successfully: {}", user_data.username);
                Ok(())
            }
            Err(e) => {
                log::error!("Error creating user: {}", e);
                Err(e)
            }
        }
    }

    // Função para obter as credenciais do usuário para autenticação
    pub async fn get_user_credentials(
        client: &Client,
        username: &str,
    ) -> Result<Option<(Uuid, String)>, tokio_postgres::Error> {
        let query = "SELECT id, password FROM users WHERE username = $1";
        log::info!("Fetching credentials for username: {}", username);

        let row = match client.query_opt(query, &[&username]).await {
            Ok(row) => row,
            Err(e) => {
                log::error!("Error fetching credentials for username: {}", e);
                return Err(e);
            }
        };

        match row {
            Some(row) => {
                let user_id: Uuid = row.get(0);
                let password: String = row.get(1);
                log::info!("Fetched credentials for username: {}", username);
                Ok(Some((user_id, password)))
            }
            None => {
                log::warn!("No credentials found for username: {}", username);
                Ok(None)
            }
        }
    }



pub async fn create_session(
    client: &Client,
    user_id: Uuid,
    token: &str,
) -> Result<(), tokio_postgres::Error> {
    let expires_at: DateTime<Utc> = Utc::now() + chrono::Duration::days(30);
    let query = "INSERT INTO sessions (user_id, token, expires_at) VALUES ($1, $2, $3)";
    
    log::info!("Creating session for user_id: {}, token: {}", user_id, token);

    match client.execute(query, &[&user_id, &token, &expires_at]).await {
        Ok(_) => {
            log::info!("Session created successfully for user_id: {}", user_id);
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to create session: {}", e);
            Err(e)
        }
    }
}

    // Função para verificar se o token da sessão é válido
    pub async fn verify_session_token(
        client: &Client,
        token: &str,
    ) -> Result<Option<Uuid>, tokio_postgres::Error> {
        let query = "SELECT user_id FROM sessions WHERE token = $1 AND expires_at > NOW()";
        log::info!("Verifying session token: {}", token);

        let row = match client.query_opt(query, &[&token]).await {
            Ok(row) => row,
            Err(e) => {
                log::error!("Error verifying session token: {}", e);
                return Err(e);
            }
        };

        match row {
            Some(row) => {
                let user_id: Uuid = row.get(0);
                log::info!("Session token valid for user_id: {}", user_id);
                Ok(Some(user_id))
            }
            None => {
                log::warn!("Invalid or expired session token: {}", token);
                Ok(None)
            }
        }
    }
}
