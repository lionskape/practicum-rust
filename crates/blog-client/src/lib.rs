pub mod error;
pub mod grpc_client;
pub mod http_client;
pub mod types;

use error::BlogClientError;
use types::{AuthResponse, ListPostsResponse, Post};

pub enum Transport {
    Http(String),
    Grpc(String),
}

pub enum BlogClient {
    Http(http_client::HttpBlogClient),
    Grpc(grpc_client::GrpcBlogClient),
}

impl BlogClient {
    pub async fn new(transport: Transport) -> Result<Self, BlogClientError> {
        match transport {
            Transport::Http(url) => Ok(BlogClient::Http(http_client::HttpBlogClient::new(&url))),
            Transport::Grpc(addr) => {
                Ok(BlogClient::Grpc(grpc_client::GrpcBlogClient::connect(&addr).await?))
            }
        }
    }

    pub fn set_token(&mut self, token: String) {
        match self {
            BlogClient::Http(c) => c.set_token(token),
            BlogClient::Grpc(c) => c.set_token(token),
        }
    }

    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        match self {
            BlogClient::Http(c) => c.register(username, email, password).await,
            BlogClient::Grpc(c) => c.register(username, email, password).await,
        }
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        match self {
            BlogClient::Http(c) => c.login(username, password).await,
            BlogClient::Grpc(c) => c.login(username, password).await,
        }
    }

    pub async fn create_post(
        &mut self,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        match self {
            BlogClient::Http(c) => c.create_post(title, content).await,
            BlogClient::Grpc(c) => c.create_post(title, content).await,
        }
    }

    pub async fn get_post(&mut self, id: i64) -> Result<Post, BlogClientError> {
        match self {
            BlogClient::Http(c) => c.get_post(id).await,
            BlogClient::Grpc(c) => c.get_post(id).await,
        }
    }

    pub async fn update_post(
        &mut self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        match self {
            BlogClient::Http(c) => c.update_post(id, title, content).await,
            BlogClient::Grpc(c) => c.update_post(id, title, content).await,
        }
    }

    pub async fn delete_post(&mut self, id: i64) -> Result<(), BlogClientError> {
        match self {
            BlogClient::Http(c) => c.delete_post(id).await,
            BlogClient::Grpc(c) => c.delete_post(id).await,
        }
    }

    pub async fn list_posts(
        &mut self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ListPostsResponse, BlogClientError> {
        match self {
            BlogClient::Http(c) => c.list_posts(limit, offset).await,
            BlogClient::Grpc(c) => c.list_posts(limit, offset).await,
        }
    }
}
