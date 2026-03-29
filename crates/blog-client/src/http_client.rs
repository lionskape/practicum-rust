use reqwest::Client;
use serde_json::json;

use crate::{
    error::BlogClientError,
    types::{AuthResponse, ErrorResponse, ListPostsResponse, Post},
};

pub struct HttpBlogClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl HttpBlogClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        }
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api{path}", self.base_url)
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }

    async fn check_error(
        &self,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, BlogClientError> {
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<ErrorResponse>(&body).map(|e| e.error).unwrap_or(body);
        Err(BlogClientError::Api { status, message })
    }

    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let resp = self
            .client
            .post(self.url("/auth/register"))
            .json(&json!({ "username": username, "email": email, "password": password }))
            .send()
            .await?;

        let resp = self.check_error(resp).await?;
        let auth: AuthResponse =
            resp.json().await.map_err(|e| BlogClientError::Deserialization(e.to_string()))?;
        self.token = Some(auth.token.clone());
        Ok(auth)
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let resp = self
            .client
            .post(self.url("/auth/login"))
            .json(&json!({ "username": username, "password": password }))
            .send()
            .await?;

        let resp = self.check_error(resp).await?;
        let auth: AuthResponse =
            resp.json().await.map_err(|e| BlogClientError::Deserialization(e.to_string()))?;
        self.token = Some(auth.token.clone());
        Ok(auth)
    }

    pub async fn create_post(&self, title: &str, content: &str) -> Result<Post, BlogClientError> {
        let mut req = self.client.post(self.url("/posts")).json(&json!({
            "title": title,
            "content": content,
        }));

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;
        let resp = self.check_error(resp).await?;
        resp.json().await.map_err(|e| BlogClientError::Deserialization(e.to_string()))
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, BlogClientError> {
        let resp = self.client.get(self.url(&format!("/posts/{id}"))).send().await?;
        let resp = self.check_error(resp).await?;
        resp.json().await.map_err(|e| BlogClientError::Deserialization(e.to_string()))
    }

    pub async fn update_post(
        &self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let mut req = self.client.put(self.url(&format!("/posts/{id}"))).json(&json!({
            "title": title,
            "content": content,
        }));

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;
        let resp = self.check_error(resp).await?;
        resp.json().await.map_err(|e| BlogClientError::Deserialization(e.to_string()))
    }

    pub async fn delete_post(&self, id: i64) -> Result<(), BlogClientError> {
        let mut req = self.client.delete(self.url(&format!("/posts/{id}")));

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;
        self.check_error(resp).await?;
        Ok(())
    }

    pub async fn list_posts(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ListPostsResponse, BlogClientError> {
        let mut url = self.url("/posts");
        let mut params = vec![];
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if let Some(o) = offset {
            params.push(format!("offset={o}"));
        }
        if !params.is_empty() {
            url = format!("{url}?{}", params.join("&"));
        }

        let resp = self.client.get(&url).send().await?;
        let resp = self.check_error(resp).await?;
        resp.json().await.map_err(|e| BlogClientError::Deserialization(e.to_string()))
    }
}
