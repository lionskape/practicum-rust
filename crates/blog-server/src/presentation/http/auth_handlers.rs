use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{domain::error::AppError, presentation::http::state::AppState};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let result = state.auth_service.register(&req.username, &req.email, &req.password).await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token: result.token,
            user: UserInfo {
                id: result.user.id,
                username: result.user.username,
                email: result.user.email,
            },
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let result = state.auth_service.login(&req.username, &req.password).await?;

    Ok(Json(AuthResponse {
        token: result.token,
        user: UserInfo {
            id: result.user.id,
            username: result.user.username,
            email: result.user.email,
        },
    }))
}
