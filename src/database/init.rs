// init.rs

use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use dotenv::dotenv;
use log::{error, info};
use std::env;
use thiserror::Error;
use tokio_postgres::{Config as PgConfig, NoTls};

use crate::database::migrations::apply_migrations;

/// Custom error types for database operations
/// Database-related error types
#[derive(Error, Debug)]
pub enum DbError {
    /// Error when required environment variable is not found
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    /// Error when DATABASE_URL parsing fails
    #[error("Failed to parse DATABASE_URL: {0}")]
    ParseError(String),

    /// Error when connection pool creation fails
    #[error("Failed to create pool: {0}")]
    PoolCreationError(String),

    /// Error when database migration fails
    #[error("Migration error: {0}")]
    MigrationError(String),
}

/// Loads the DATABASE_URL from environment variables
/// Returns: Result containing the database URL string or an error
fn load_database_url() -> Result<String, DbError> {
    dotenv().ok();
    env::var("DATABASE_URL").map_err(|_| DbError::EnvVarNotFound("DATABASE_URL".to_string()))
}

/// Creates a connection pool using the provided database URL
/// 
/// # Arguments
/// * `database_url` - The PostgreSQL connection string
/// 
/// # Returns
/// * `Result<Pool, DbError>` - A connection pool or an error
fn create_pool(database_url: &str) -> Result<Pool, DbError> {
    // Parse the database URL into a PostgreSQL configuration
    let pg_config = database_url
        .parse::<PgConfig>()
        .map_err(|e| DbError::ParseError(e.to_string()))?;

    // Convert tokio-postgres config to deadpool config
    let mut cfg = Config::new();
    
    // Set configuration parameters from parsed URL
    cfg.user = pg_config.get_user().map(ToString::to_string);
    cfg.password = pg_config
        .get_password()
        .map(|s| String::from_utf8(s.to_vec()))
        .transpose()
        .map_err(|e| DbError::ParseError(e.to_string()))?;
    cfg.dbname = pg_config.get_dbname().map(ToString::to_string);
    cfg.host = pg_config.get_hosts().first().and_then(|host| match host {
        tokio_postgres::config::Host::Tcp(host) => Some(host.to_string()),
        _ => None,
    });
    cfg.port = pg_config.get_ports().first().copied();

    // Configure connection pool manager with fast recycling
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    // Create and return the connection pool
    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
        .map_err(|e| DbError::PoolCreationError(e.to_string()))
}

/// Initializes the database connection pool and applies migrations
/// 
/// # Returns
/// * `Result<Pool, DbError>` - Initialized connection pool or an error
pub async fn init_db() -> Result<Pool, DbError> {
    let database_url = load_database_url()?;
    let pool = create_pool(&database_url)?;

    // Get a connection and apply migrations
    let client = pool
        .get()
        .await
        .map_err(|e| DbError::MigrationError(e.to_string()))?;

    apply_migrations(&client)
        .await
        .map_err(|e| DbError::MigrationError(e.to_string()))?;

    info!("✓ Database pool initialized and migrations applied successfully");
    Ok(pool)
}