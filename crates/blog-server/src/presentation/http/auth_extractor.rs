use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{domain::error::AppError, presentation::http::state::AppState};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub username: String,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;

        let claims = state.jwt.validate_token(token)?;

        Ok(AuthenticatedUser { user_id: claims.user_id, username: claims.username })
    }
}
