use deadpool_postgres::Client;

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

async fn create_tables(client: &Client) -> Result<(), String> {
    // Enable UUID extension
    let enable_uuid = "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"";
    client
        .execute(enable_uuid, &[])
        .await
        .map_err(|e| format!("Error enabling uuid: {}", e))?;

    // Users table
    let create_users = "
        CREATE TABLE IF NOT EXISTS users (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            username VARCHAR(255) NOT NULL UNIQUE,
            password VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL UNIQUE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )";
    client
        .execute(create_users, &[])
        .await
        .map_err(|e| format!("Error creating users: {}", e))?;

    // Chats table
    let create_chats = "
        CREATE TABLE IF NOT EXISTS chats (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            name VARCHAR(255),
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )";
    client
        .execute(create_chats, &[])
        .await
        .map_err(|e| format!("Error creating chats: {}", e))?;

    // Chat members table
    let create_members = "
        CREATE TABLE IF NOT EXISTS chat_members (
            chat_id UUID REFERENCES chats(id) ON DELETE CASCADE,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            status TEXT NOT NULL,
            is_creator BOOLEAN NOT NULL DEFAULT FALSE,
            joined_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (chat_id, user_id)
        )";
    client
        .execute(create_members, &[])
        .await
        .map_err(|e| format!("Error creating members: {}", e))?;

    // Chat keys table
    let create_chat_keys_table = "
        CREATE TABLE IF NOT EXISTS chat_keys (
            chat_id UUID REFERENCES chats(id),
            user_id UUID REFERENCES users(id),
            encrypted_key BYTEA NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (chat_id, user_id)
        )
    ";

    // Encrypted messages table
    let create_encrypted_messages_table = "
        CREATE TABLE IF NOT EXISTS encrypted_messages (
            id UUID PRIMARY KEY,
            chat_id UUID REFERENCES chats(id),
            sender_id UUID REFERENCES users(id),
            encrypted_content BYTEA NOT NULL,
            nonce BYTEA NOT NULL,
            key_fingerprint VARCHAR(255),
            timestamp TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (chat_id, sender_id) REFERENCES chat_members(chat_id, user_id)
        )
    ";

    // Execute the queries
    client
        .batch_execute(&format!(
            "{}; {};",
            create_chat_keys_table, create_encrypted_messages_table
        ))
        .await
        .map_err(|e| e.to_string())?;

    // Sessions table
    let create_sessions = "
        CREATE TABLE IF NOT EXISTS sessions (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            token TEXT NOT NULL UNIQUE,
            expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )";
    client
        .execute(create_sessions, &[])
        .await
        .map_err(|e| format!("Error creating sessions: {}", e))?;

    // Invites table
    let create_invites = "
        CREATE TABLE IF NOT EXISTS invites (
            id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
            chat_id UUID REFERENCES chats(id) ON DELETE CASCADE,
            inviter_id UUID REFERENCES users(id) ON DELETE CASCADE,
            invitee_id UUID REFERENCES users(id) ON DELETE CASCADE,
            status VARCHAR(20) NOT NULL DEFAULT 'pending',
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )";
    client
        .execute(create_invites, &[])
        .await
        .map_err(|e| format!("Error creating invites: {}", e))?;

    Ok(())
}

