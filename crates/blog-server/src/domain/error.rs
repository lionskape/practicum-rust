use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("user not found")]
    UserNotFound,

    #[error("user already exists")]
    UserAlreadyExists,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("post not found")]
    PostNotFound,

    #[error("forbidden: you are not the author")]
    Forbidden,

    #[error("unauthorized")]
    Unauthorized,

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::PostNotFound,
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                AppError::UserAlreadyExists
            }
            other => AppError::Internal(other.to_string()),
        }
    }
}
