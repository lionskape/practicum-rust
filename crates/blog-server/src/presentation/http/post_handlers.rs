use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{error::AppError, post::PostWithAuthor},
    presentation::http::{auth_extractor::AuthenticatedUser, state::AppState},
};

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub author_username: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListPostsResponse {
    pub posts: Vec<PostResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

fn post_to_response(post: &PostWithAuthor) -> PostResponse {
    PostResponse {
        id: post.id,
        title: post.title.clone(),
        content: post.content.clone(),
        author_id: post.author_id,
        author_username: post.author_username.clone(),
        created_at: post.created_at.to_rfc3339(),
        updated_at: post.updated_at.to_rfc3339(),
    }
}

pub async fn create_post(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreatePostRequest>,
) -> Result<(StatusCode, Json<PostResponse>), AppError> {
    let post = state.blog_service.create_post(&req.title, &req.content, user.user_id).await?;
    Ok((StatusCode::CREATED, Json(post_to_response(&post))))
}

pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<PostResponse>, AppError> {
    let post = state.blog_service.get_post(id).await?;
    Ok(Json(post_to_response(&post)))
}

pub async fn update_post(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePostRequest>,
) -> Result<Json<PostResponse>, AppError> {
    let post = state.blog_service.update_post(id, &req.title, &req.content, user.user_id).await?;
    Ok(Json(post_to_response(&post)))
}

pub async fn delete_post(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    state.blog_service.delete_post(id, user.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_posts(
    State(state): State<AppState>,
    Query(query): Query<ListPostsQuery>,
) -> Result<Json<ListPostsResponse>, AppError> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    let (posts, total) = state.blog_service.list_posts(limit, offset).await?;

    Ok(Json(ListPostsResponse {
        posts: posts.iter().map(post_to_response).collect(),
        total,
        limit,
        offset,
    }))
}
