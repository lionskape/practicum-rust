use crate::{
    data::post_repository::PostRepository,
    domain::{error::AppError, post::PostWithAuthor},
};

pub struct BlogService<P: PostRepository> {
    post_repo: P,
}

impl<P: PostRepository> BlogService<P> {
    pub fn new(post_repo: P) -> Self {
        Self { post_repo }
    }

    pub async fn create_post(
        &self,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> Result<PostWithAuthor, AppError> {
        self.post_repo.create(title, content, author_id).await
    }

    pub async fn get_post(&self, id: i64) -> Result<PostWithAuthor, AppError> {
        self.post_repo.find_by_id(id).await
    }

    pub async fn update_post(
        &self,
        id: i64,
        title: &str,
        content: &str,
        user_id: i64,
    ) -> Result<PostWithAuthor, AppError> {
        let post = self.post_repo.find_raw_by_id(id).await?;
        if post.author_id != user_id {
            return Err(AppError::Forbidden);
        }
        self.post_repo.update(id, title, content).await
    }

    pub async fn delete_post(&self, id: i64, user_id: i64) -> Result<(), AppError> {
        let post = self.post_repo.find_raw_by_id(id).await?;
        if post.author_id != user_id {
            return Err(AppError::Forbidden);
        }
        self.post_repo.delete(id).await
    }

    pub async fn list_posts(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<PostWithAuthor>, i64), AppError> {
        let limit = if limit <= 0 { 20 } else { limit.min(100) };
        let offset = if offset < 0 { 0 } else { offset };
        self.post_repo.list(limit, offset).await
    }
}
