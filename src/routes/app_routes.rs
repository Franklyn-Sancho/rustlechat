// src/router.rs

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::handlers::chat_handlers::{create_chat, get_chat_messages, send_message};
use crate::handlers::{self, auth_handlers};
use crate::handlers::websocker_handlers::{websocket_handler, AppState};
use crate::middleware::{self, auth_middleware, ws_auth_middleware};
use crate::{database::init::DbClient};
use axum::middleware::from_fn;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use tower_http::trace::TraceLayer;
use crate::routes::app_routes::ws_auth_middleware::ws_auth_middleware;
use crate::routes::app_routes::auth_middleware::auth_middleware;

pub fn create_router(db: DbClient) -> Router {
    let connections = Arc::new(Mutex::new(HashMap::new()));
    let state = AppState {
        connections,
        db, 
        current_user_id: None,
    };

    Router::new()
        .route("/", get(|| async { "Hello, world!" }))
        .route("/register", post(auth_handlers::register))
        .route("/login", post(auth_handlers::login))
        .route(
            "/ws",
            get(websocket_handler)
                .route_layer(from_fn(ws_auth_middleware)),
        )
        .route("/create_chat", post(create_chat).route_layer(from_fn(auth_middleware)))
        .route("/get_messages/:chat_id", get(get_chat_messages).route_layer(from_fn(auth_middleware)))
        .route("/send_message", post(send_message).route_layer(from_fn(auth_middleware)))
        .layer(TraceLayer::new_for_http())
        .layer(Extension(state))
}
