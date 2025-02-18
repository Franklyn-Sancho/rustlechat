use dotenv::dotenv;
use std::{env, sync::Arc};
use tokio_postgres::{Client, Config, NoTls};

use crate::database::migrations::apply_migrations;

pub type DbClient = Arc<Client>; 

// This function initializes the database connection.
pub async fn init_db() -> Result<DbClient, String> {
    dotenv().ok(); // Load environment variables from a `.env` file.

    // Retrieve the DATABASE_URL environment variable. If not set, return an error.
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL not set".to_string())?;

    // Parse the DATABASE_URL into a Config object, handling any parsing errors.
    let config = database_url
        .parse::<Config>()
        .map_err(|e| format!("Failed to parse DATABASE_URL: {}", e))?;

    println!("Connecting to database: {}", database_url); // Log the connection URL (for debugging purposes).

    // Establish the database connection using the config object and `NoTls` (no TLS encryption).
    let (client, connection) = config
        .connect(NoTls)
        .await
        .map_err(|e| format!("Failed to connect to the database: {}", e))?;

    // Spawn a separate asynchronous task to manage the connection in the background.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e); // Log any connection errors.
        }
    });

    // Apply any pending migrations to the database.
    apply_migrations(&client).await?;

    // Return the client wrapped in an Arc, making it shareable across threads.
    Ok(Arc::new(client))
}
