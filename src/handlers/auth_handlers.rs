// handlers/auth_handler.rs

use axum::{
    debug_handler,
    extract::Extension,
    response::IntoResponse,
    Json,
};
use crate::{
    app_state::AppState,
    models::user::{LoginData, RegisterData},
    services::auth_service,
};

/// Handler for user registration
#[debug_handler]
pub async fn register(
    Extension(state): Extension<AppState>,
    Json(payload): Json<RegisterData>,
) -> impl IntoResponse {
    auth_service::register_user(state.db.clone(), payload).await
}

/// Handler for user login
#[debug_handler]
pub async fn login(
    Extension(state): Extension<AppState>,
    Json(payload): Json<LoginData>,
) -> impl IntoResponse {
    auth_service::login_user(state.db.clone(), payload).await
}

