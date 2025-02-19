use axum::{
    debug_handler,
    extract::Extension,
    response::IntoResponse,
    Json,
};
use crate::{
    app_state::AppState,
    models::user::{LoginData, RegisterData},
    services::auth_service::AuthService,
};

/// Handler for user registration
#[debug_handler]
pub async fn register(
    Extension(state): Extension<AppState>,
    Json(payload): Json<RegisterData>,
) -> impl IntoResponse {
    let auth_service = AuthService::new(state.db.clone());
    auth_service.register_user(payload).await
}

/// Handler for user login
#[debug_handler]
pub async fn login(
    Extension(state): Extension<AppState>,
    Json(payload): Json<LoginData>,
) -> impl IntoResponse {
    let auth_service = AuthService::new(state.db.clone());
    auth_service.login_user(payload).await
}


