use sqlx::PgPool;

use crate::domain::{
    error::AppError,
    post::{Post, PostWithAuthor},
};

pub trait PostRepository: Send + Sync {
    fn find_by_id(&self, id: i64) -> impl Future<Output = Result<PostWithAuthor, AppError>> + Send;
    fn create(
        &self,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> impl Future<Output = Result<PostWithAuthor, AppError>> + Send;
    fn update(
        &self,
        id: i64,
        title: &str,
        content: &str,
    ) -> impl Future<Output = Result<PostWithAuthor, AppError>> + Send;
    fn delete(&self, id: i64) -> impl Future<Output = Result<(), AppError>> + Send;
    fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> impl Future<Output = Result<(Vec<PostWithAuthor>, i64), AppError>> + Send;
    fn find_raw_by_id(&self, id: i64) -> impl Future<Output = Result<Post, AppError>> + Send;
}

#[derive(Clone)]
pub struct PgPostRepository {
    pool: PgPool,
}

impl PgPostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PostRepository for PgPostRepository {
    async fn find_by_id(&self, id: i64) -> Result<PostWithAuthor, AppError> {
        sqlx::query_as::<_, PostWithAuthor>(
            "SELECT p.id, p.title, p.content, p.author_id, u.username AS author_username, \
             p.created_at, p.updated_at \
             FROM posts p JOIN users u ON p.author_id = u.id WHERE p.id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::PostNotFound,
            other => AppError::Internal(other.to_string()),
        })
    }

    async fn create(
        &self,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> Result<PostWithAuthor, AppError> {
        let post = sqlx::query_as::<_, Post>(
            "INSERT INTO posts (title, content, author_id) VALUES ($1, $2, $3) \
             RETURNING id, title, content, author_id, created_at, updated_at",
        )
        .bind(title)
        .bind(content)
        .bind(author_id)
        .fetch_one(&self.pool)
        .await?;

        self.find_by_id(post.id).await
    }

    async fn update(
        &self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<PostWithAuthor, AppError> {
        let result = sqlx::query(
            "UPDATE posts SET title = $1, content = $2, updated_at = now() WHERE id = $3",
        )
        .bind(title)
        .bind(content)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::PostNotFound);
        }

        self.find_by_id(id).await
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result =
            sqlx::query("DELETE FROM posts WHERE id = $1").bind(id).execute(&self.pool).await?;

        if result.rows_affected() == 0 {
            return Err(AppError::PostNotFound);
        }

        Ok(())
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<(Vec<PostWithAuthor>, i64), AppError> {
        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM posts").fetch_one(&self.pool).await?;

        let posts = sqlx::query_as::<_, PostWithAuthor>(
            "SELECT p.id, p.title, p.content, p.author_id, u.username AS author_username, \
             p.created_at, p.updated_at \
             FROM posts p JOIN users u ON p.author_id = u.id \
             ORDER BY p.created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((posts, total.0))
    }

    async fn find_raw_by_id(&self, id: i64) -> Result<Post, AppError> {
        sqlx::query_as::<_, Post>(
            "SELECT id, title, content, author_id, created_at, updated_at FROM posts WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::PostNotFound,
            other => AppError::Internal(other.to_string()),
        })
    }
}
