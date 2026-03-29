use tonic::{metadata::MetadataValue, transport::Channel};

use crate::{
    error::BlogClientError,
    types::{AuthResponse, ListPostsResponse, Post, UserInfo},
};

mod proto {
    tonic::include_proto!("blog");
}

use proto::blog_service_client::BlogServiceClient;

pub struct GrpcBlogClient {
    client: BlogServiceClient<Channel>,
    token: Option<String>,
}

impl GrpcBlogClient {
    pub async fn connect(addr: &str) -> Result<Self, BlogClientError> {
        let client = BlogServiceClient::connect(addr.to_string()).await?;
        Ok(Self { client, token: None })
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    fn add_auth<T>(&self, mut req: tonic::Request<T>) -> tonic::Request<T> {
        if let Some(token) = &self.token {
            let val = format!("Bearer {token}");
            if let Ok(meta) = val.parse::<MetadataValue<tonic::metadata::Ascii>>() {
                req.metadata_mut().insert("authorization", meta);
            }
        }
        req
    }

    fn convert_user_info(info: Option<proto::UserInfo>) -> UserInfo {
        match info {
            Some(u) => UserInfo { id: u.id, username: u.username, email: u.email },
            None => UserInfo { id: 0, username: String::new(), email: String::new() },
        }
    }

    fn convert_post(p: proto::PostResponse) -> Post {
        Post {
            id: p.id,
            title: p.title,
            content: p.content,
            author_id: p.author_id,
            author_username: p.author_username,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }

    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let req = tonic::Request::new(proto::RegisterRequest {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        });

        let resp = self.client.register(req).await?.into_inner();
        self.token = Some(resp.token.clone());
        Ok(AuthResponse { token: resp.token, user: Self::convert_user_info(resp.user) })
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let req = tonic::Request::new(proto::LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        });

        let resp = self.client.login(req).await?.into_inner();
        self.token = Some(resp.token.clone());
        Ok(AuthResponse { token: resp.token, user: Self::convert_user_info(resp.user) })
    }

    pub async fn create_post(
        &mut self,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let req = self.add_auth(tonic::Request::new(proto::CreatePostRequest {
            title: title.to_string(),
            content: content.to_string(),
        }));

        let resp = self.client.create_post(req).await?.into_inner();
        Ok(Self::convert_post(resp))
    }

    pub async fn get_post(&mut self, id: i64) -> Result<Post, BlogClientError> {
        let req = tonic::Request::new(proto::GetPostRequest { id });
        let resp = self.client.get_post(req).await?.into_inner();
        Ok(Self::convert_post(resp))
    }

    pub async fn update_post(
        &mut self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let req = self.add_auth(tonic::Request::new(proto::UpdatePostRequest {
            id,
            title: title.to_string(),
            content: content.to_string(),
        }));

        let resp = self.client.update_post(req).await?.into_inner();
        Ok(Self::convert_post(resp))
    }

    pub async fn delete_post(&mut self, id: i64) -> Result<(), BlogClientError> {
        let req = self.add_auth(tonic::Request::new(proto::DeletePostRequest { id }));
        self.client.delete_post(req).await?;
        Ok(())
    }

    pub async fn list_posts(
        &mut self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ListPostsResponse, BlogClientError> {
        let req = tonic::Request::new(proto::ListPostsRequest {
            limit: limit.unwrap_or(20),
            offset: offset.unwrap_or(0),
        });

        let resp = self.client.list_posts(req).await?.into_inner();
        Ok(ListPostsResponse {
            posts: resp.posts.into_iter().map(Self::convert_post).collect(),
            total: resp.total,
            limit: resp.limit,
            offset: resp.offset,
        })
    }
}
