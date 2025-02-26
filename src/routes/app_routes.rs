use std::sync::Arc;
use crate::app_state::AppState;
use crate::crypto::{MessageCrypto, ChatKey};
use crate::handlers::auth_handlers;
use crate::handlers::chat_handlers::{create_chat, get_chat_messages, send_message_handler};
use crate::handlers::invitation_handlers::respond_to_invitation;
use crate::middleware::auth_middleware::auth_middleware;
use crate::middleware::ws_auth_middleware::ws_auth_middleware;
use crate::websocket::connection_manager::ConnectionManager;
use crate::websocket::handlers::websocket_handler;
use axum::middleware::from_fn;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use deadpool_postgres::Pool;
use tower_http::trace::TraceLayer;

pub fn create_router(db: Pool) -> Router {
    // Initialize ConnectionManager with database pool
    let connections = ConnectionManager::new(db.clone());
    
    // Generate a new ChatKey for the application
    let chat_key = ChatKey::generate();
    
    // Initialize MessageCrypto with the generated key
    let crypto = match MessageCrypto::new(&chat_key) {
        Ok(crypto) => Arc::new(crypto),
        Err(e) => {
            eprintln!("Failed to initialize MessageCrypto: {}", e);
            panic!("Failed to initialize encryption");
        }
    };

    // Create AppState with ConnectionManager, database pool and crypto
    let state = AppState {
        connections,
        db,
        crypto,
        current_user_id: None,
    };

    Router::new()
        .route("/", get(|| async { "Hello, world!" }))
        .route("/register", post(auth_handlers::register))
        .route("/login", post(auth_handlers::login))
        .route(
            "/ws",
            get(websocket_handler).route_layer(from_fn(ws_auth_middleware)),
        )
        .route(
            "/create_chat",
            post(create_chat).route_layer(from_fn(auth_middleware)),
        )
        .route(
            "/get_messages/:chat_id",
            get(get_chat_messages).route_layer(from_fn(auth_middleware)),
        )
        .route(
            "/send_message",
            post(send_message_handler).route_layer(from_fn(auth_middleware)),
        )
        .route(
            "/invites/respond",
            post(respond_to_invitation).route_layer(from_fn(auth_middleware)),
        )
        .layer(TraceLayer::new_for_http())
        .layer(Extension(state))
}
