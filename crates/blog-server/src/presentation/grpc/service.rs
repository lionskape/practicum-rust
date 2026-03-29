use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::{
    application::{auth_service::AuthService, blog_service::BlogService},
    data::{post_repository::PgPostRepository, user_repository::PgUserRepository},
    domain::error::AppError,
    infrastructure::jwt::JwtService,
    presentation::grpc::proto::{
        AuthResponse, CreatePostRequest, DeletePostRequest, DeleteResponse, GetPostRequest,
        ListPostsRequest, ListPostsResponse, LoginRequest, PostResponse, RegisterRequest,
        UpdatePostRequest, UserInfo, blog_service_server::BlogService as BlogServiceTrait,
    },
};

pub struct BlogGrpcService {
    auth_service: Arc<AuthService<PgUserRepository>>,
    blog_service: Arc<BlogService<PgPostRepository>>,
    jwt: Arc<JwtService>,
}

impl BlogGrpcService {
    pub fn new(
        auth_service: Arc<AuthService<PgUserRepository>>,
        blog_service: Arc<BlogService<PgPostRepository>>,
        jwt: Arc<JwtService>,
    ) -> Self {
        Self { auth_service, blog_service, jwt }
    }

    #[allow(clippy::result_large_err)]
    fn extract_user_id(
        &self,
        req: &Request<impl std::fmt::Debug>,
    ) -> Result<(i64, String), Status> {
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("missing or invalid authorization header"))?;

        let claims =
            self.jwt.validate_token(token).map_err(|_| Status::unauthenticated("invalid token"))?;

        Ok((claims.user_id, claims.username))
    }
}

impl From<AppError> for Status {
    fn from(err: AppError) -> Self {
        match err {
            AppError::UserNotFound => Status::not_found(err.to_string()),
            AppError::UserAlreadyExists => Status::already_exists(err.to_string()),
            AppError::InvalidCredentials => Status::unauthenticated(err.to_string()),
            AppError::PostNotFound => Status::not_found(err.to_string()),
            AppError::Forbidden => Status::permission_denied(err.to_string()),
            AppError::Unauthorized => Status::unauthenticated(err.to_string()),
            AppError::Internal(msg) => Status::internal(msg),
        }
    }
}

#[tonic::async_trait]
impl BlogServiceTrait for BlogGrpcService {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .auth_service
            .register(&req.username, &req.email, &req.password)
            .await
            .map_err(Status::from)?;

        Ok(Response::new(AuthResponse {
            token: result.token,
            user: Some(UserInfo {
                id: result.user.id,
                username: result.user.username,
                email: result.user.email,
            }),
        }))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let result =
            self.auth_service.login(&req.username, &req.password).await.map_err(Status::from)?;

        Ok(Response::new(AuthResponse {
            token: result.token,
            user: Some(UserInfo {
                id: result.user.id,
                username: result.user.username,
                email: result.user.email,
            }),
        }))
    }

    async fn create_post(
        &self,
        request: Request<CreatePostRequest>,
    ) -> Result<Response<PostResponse>, Status> {
        let (user_id, _) = self.extract_user_id(&request)?;
        let req = request.into_inner();
        let post = self
            .blog_service
            .create_post(&req.title, &req.content, user_id)
            .await
            .map_err(Status::from)?;

        Ok(Response::new(PostResponse {
            id: post.id,
            title: post.title,
            content: post.content,
            author_id: post.author_id,
            author_username: post.author_username,
            created_at: post.created_at.to_rfc3339(),
            updated_at: post.updated_at.to_rfc3339(),
        }))
    }

    async fn get_post(
        &self,
        request: Request<GetPostRequest>,
    ) -> Result<Response<PostResponse>, Status> {
        let req = request.into_inner();
        let post = self.blog_service.get_post(req.id).await.map_err(Status::from)?;

        Ok(Response::new(PostResponse {
            id: post.id,
            title: post.title,
            content: post.content,
            author_id: post.author_id,
            author_username: post.author_username,
            created_at: post.created_at.to_rfc3339(),
            updated_at: post.updated_at.to_rfc3339(),
        }))
    }

    async fn update_post(
        &self,
        request: Request<UpdatePostRequest>,
    ) -> Result<Response<PostResponse>, Status> {
        let (user_id, _) = self.extract_user_id(&request)?;
        let req = request.into_inner();
        let post = self
            .blog_service
            .update_post(req.id, &req.title, &req.content, user_id)
            .await
            .map_err(Status::from)?;

        Ok(Response::new(PostResponse {
            id: post.id,
            title: post.title,
            content: post.content,
            author_id: post.author_id,
            author_username: post.author_username,
            created_at: post.created_at.to_rfc3339(),
            updated_at: post.updated_at.to_rfc3339(),
        }))
    }

    async fn delete_post(
        &self,
        request: Request<DeletePostRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let (user_id, _) = self.extract_user_id(&request)?;
        let req = request.into_inner();
        self.blog_service.delete_post(req.id, user_id).await.map_err(Status::from)?;
        Ok(Response::new(DeleteResponse {}))
    }

    async fn list_posts(
        &self,
        request: Request<ListPostsRequest>,
    ) -> Result<Response<ListPostsResponse>, Status> {
        let req = request.into_inner();
        let (posts, total) =
            self.blog_service.list_posts(req.limit, req.offset).await.map_err(Status::from)?;

        let post_responses = posts
            .into_iter()
            .map(|p| PostResponse {
                id: p.id,
                title: p.title,
                content: p.content,
                author_id: p.author_id,
                author_username: p.author_username,
                created_at: p.created_at.to_rfc3339(),
                updated_at: p.updated_at.to_rfc3339(),
            })
            .collect();

        Ok(Response::new(ListPostsResponse {
            posts: post_responses,
            total,
            limit: req.limit,
            offset: req.offset,
        }))
    }
}
