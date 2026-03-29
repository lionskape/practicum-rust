use sqlx::PgPool;

use crate::domain::{error::AppError, user::User};

#[allow(dead_code)]
pub trait UserRepository: Send + Sync {
    fn find_by_id(&self, id: i64) -> impl Future<Output = Result<User, AppError>> + Send;
    fn find_by_username(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<User, AppError>> + Send;
    fn create(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> impl Future<Output = Result<User, AppError>> + Send;
}

#[derive(Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: i64) -> Result<User, AppError> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::UserNotFound,
            other => AppError::Internal(other.to_string()),
        })
    }

    async fn find_by_username(&self, username: &str) -> Result<User, AppError> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::UserNotFound,
            other => AppError::Internal(other.to_string()),
        })
    }

    async fn create(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<User, AppError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id, username, email, password_hash, created_at",
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::from)
    }
}
