use axum::{debug_handler, response::IntoResponse, Extension, Json};
use std::sync::Arc;
use tokio_postgres::Client;

use crate::{
    models::user::{LoginData, RegisterData},
    services::auth_service, websocket::types::AppState,
};

pub async fn register(
    Extension(state): Extension<AppState>,  // Receives AppState now
    Json(payload): Json<RegisterData>,
) -> impl IntoResponse {
    // Calls the service function to register the user
    auth_service::register_user(state.db.clone(), payload).await      // Uses state.db
}

#[debug_handler]
pub async fn login(
    Extension(state): Extension<AppState>,  // Receives AppState now
    Json(payload): Json<LoginData>,
) -> impl IntoResponse {
    auth_service::login_user(state.db.clone(), payload).await  // Uses state.db
}