// Import necessary modules
mod handlers;
mod middleware;
mod models;
mod services;
mod database;
mod routes;
mod websocket;
mod app_state;
mod repositories;
mod utils;

// Import required functions and types
use database::init::init_db;
use routes::app_routes::create_router;
use std::net::SocketAddr;
use tokio::signal;
use std::sync::Arc; // Import Arc

// The main entry point for the application using the tokio runtime.
#[tokio::main]
async fn main() {
    // Initialize the logger for logging messages
    env_logger::init();

    // Initialize the database connection and handle errors
    let db = match init_db().await {
        Ok(db) => {
            println!("Database initialized successfully!");  // Log successful initialization
            db
        }
        Err(e) => {
            eprintln!("Error initializing the database: {}", e);  // Log the error if initialization fails
            return;
        }
    };

    // Wrap the Pool in an Arc
    let db = Arc::new(db);

    // Create the router using the function from the router module
    let app = create_router(db);

    // Set the server address to listen on all IP addresses (0.0.0.0) and port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running on http://{}", addr);  // Log server address

    // Start the server, binding to the specified address and enabling graceful shutdown
    axum::Server::bind(&addr)
        .serve(app.into_make_service())  // Convert the app into a service
        .with_graceful_shutdown(shutdown_signal())  // Enable graceful shutdown using the shutdown signal handler
        .await
        .unwrap();  // Panic if server fails to start
}

// A function to handle graceful shutdown by listening for termination signals.
async fn shutdown_signal() {
    // Handle Ctrl+C signal for graceful shutdown
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    // Unix-specific signal handling (e.g., SIGTERM)
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    // Wait for either Ctrl+C or the termination signal
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    // Log when shutdown signal is received and starting graceful shutdown
    println!("Signal received, starting graceful shutdown");
}
