use std::sync::Arc;

use crate::{
    application::{auth_service::AuthService, blog_service::BlogService},
    data::{post_repository::PgPostRepository, user_repository::PgUserRepository},
    infrastructure::jwt::JwtService,
};

pub type SharedAuthService = Arc<AuthService<PgUserRepository>>;
pub type SharedBlogService = Arc<BlogService<PgPostRepository>>;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: SharedAuthService,
    pub blog_service: SharedBlogService,
    pub jwt: Arc<JwtService>,
}
