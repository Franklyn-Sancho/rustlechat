use tokio_postgres::Client;



// This function applies database migrations, such as creating the database and tables.
pub async fn apply_migrations(client: &Client) -> Result<(), String> {
    // Create the database if it does not exist
    create_database_if_not_exists(client).await;

    // Create the necessary tables
    create_tables(client).await?;

    Ok(())
}

// This function checks if the database exists and creates it if necessary.
async fn create_database_if_not_exists(client: &Client) {
    let check_db_query = "SELECT 1 FROM pg_database WHERE datname = 'rustle_chat_db'";

    // Execute query to check if the database already exists
    let rows = client.query(check_db_query, &[]).await.unwrap();

    // If the database doesn't exist, create it
    if rows.is_empty() {
        let create_db_query = "CREATE DATABASE rustle_chat_db";
        client.execute(create_db_query, &[]).await.unwrap();
        println!("Database 'rustle_chat_db' created successfully");
    } else {
        println!("The database 'rustle_chat_db' already exists");
    }
}

// This function creates all the necessary tables for the application.
async fn create_tables(client: &Client) -> Result<(), String> {
    // Enable the uuid-ossp extension for generating UUIDs, if not already enabled
    let enable_uuid_extension_query = "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"";
    client
        .execute(enable_uuid_extension_query, &[])
        .await
        .map_err(|e| format!("Error enabling uuid-ossp extension: {}", e))?;

    // Create the 'users' table
    let create_users_table_query = "
        CREATE TABLE IF NOT EXISTS users (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            username VARCHAR(255) NOT NULL UNIQUE,
            password VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL UNIQUE
        )
    ";
    client
        .execute(create_users_table_query, &[])
        .await
        .map_err(|e| format!("Error creating users table: {}", e))?;

    // Create the 'chats' table
    let create_chats_table_query = "
        CREATE TABLE IF NOT EXISTS chats (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            name VARCHAR(255)
        )
    ";
    client
        .execute(create_chats_table_query, &[])
        .await
        .map_err(|e| format!("Error creating chats table: {}", e))?;

    // Create the 'chat_members' table for the many-to-many relationship between users and chats
    let create_chat_members_table_query = "
        CREATE TABLE IF NOT EXISTS chat_members (
            chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            PRIMARY KEY (chat_id, user_id)
        )
    ";
    client
        .execute(create_chat_members_table_query, &[])
        .await
        .map_err(|e| format!("Error creating chat members table: {}", e))?;

    // Create the 'messages' table to store chat messages
    let create_messages_table_query = "
        CREATE TABLE IF NOT EXISTS messages (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            chat_id UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
            sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            message_text TEXT NOT NULL,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
    ";
    client
        .execute(create_messages_table_query, &[])
        .await
        .map_err(|e| format!("Error creating messages table: {}", e))?;

    // Create the 'sessions' table to manage user login sessions
    let create_sessions_table_query = "
        CREATE TABLE IF NOT EXISTS sessions (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token TEXT NOT NULL UNIQUE,
            expires_at TIMESTAMP NOT NULL DEFAULT (NOW() + INTERVAL '30 days')
        )
    ";
    client
        .execute(create_sessions_table_query, &[])
        .await
        .map_err(|e| format!("Error creating sessions table: {}", e))?;

    Ok(())
}


