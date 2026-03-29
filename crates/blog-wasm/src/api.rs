use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::storage;

const BASE_URL: &str = "http://localhost:8080/api";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub author_username: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPostsResponse {
    pub posts: Vec<PostResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

fn auth_header() -> Option<String> {
    storage::load_token().map(|t| format!("Bearer {t}"))
}

pub async fn register(
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, JsValue> {
    let body = serde_json::json!({
        "username": username,
        "email": email,
        "password": password,
    });

    let resp = Request::post(&format!("{BASE_URL}/auth/register"))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .send()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    if !resp.ok() {
        let err: ErrorResponse =
            resp.json().await.unwrap_or(ErrorResponse { error: "Unknown error".to_string() });
        return Err(JsValue::from_str(&err.error));
    }

    let auth: AuthResponse = resp.json().await.map_err(|e| JsValue::from_str(&e.to_string()))?;
    storage::save_token(&auth.token);
    storage::save_user_id(auth.user.id);
    Ok(auth)
}

pub async fn login(username: &str, password: &str) -> Result<AuthResponse, JsValue> {
    let body = serde_json::json!({
        "username": username,
        "password": password,
    });

    let resp = Request::post(&format!("{BASE_URL}/auth/login"))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .send()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    if !resp.ok() {
        let err: ErrorResponse =
            resp.json().await.unwrap_or(ErrorResponse { error: "Invalid credentials".to_string() });
        return Err(JsValue::from_str(&err.error));
    }

    let auth: AuthResponse = resp.json().await.map_err(|e| JsValue::from_str(&e.to_string()))?;
    storage::save_token(&auth.token);
    storage::save_user_id(auth.user.id);
    Ok(auth)
}

pub async fn create_post(title: &str, content: &str) -> Result<PostResponse, JsValue> {
    let body = serde_json::json!({
        "title": title,
        "content": content,
    });

    let mut req =
        Request::post(&format!("{BASE_URL}/posts")).header("Content-Type", "application/json");

    if let Some(auth) = auth_header() {
        req = req.header("Authorization", &auth);
    }

    let resp = req
        .body(body.to_string())
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .send()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    if !resp.ok() {
        let err: ErrorResponse = resp
            .json()
            .await
            .unwrap_or(ErrorResponse { error: "Failed to create post".to_string() });
        return Err(JsValue::from_str(&err.error));
    }

    resp.json().await.map_err(|e| JsValue::from_str(&e.to_string()))
}

pub async fn update_post(id: i64, title: &str, content: &str) -> Result<PostResponse, JsValue> {
    let body = serde_json::json!({
        "title": title,
        "content": content,
    });

    let mut req =
        Request::put(&format!("{BASE_URL}/posts/{id}")).header("Content-Type", "application/json");

    if let Some(auth) = auth_header() {
        req = req.header("Authorization", &auth);
    }

    let resp = req
        .body(body.to_string())
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .send()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    if !resp.ok() {
        let err: ErrorResponse = resp
            .json()
            .await
            .unwrap_or(ErrorResponse { error: "Failed to update post".to_string() });
        return Err(JsValue::from_str(&err.error));
    }

    resp.json().await.map_err(|e| JsValue::from_str(&e.to_string()))
}

pub async fn list_posts(limit: i64, offset: i64) -> Result<ListPostsResponse, JsValue> {
    let resp = Request::get(&format!("{BASE_URL}/posts?limit={limit}&offset={offset}"))
        .send()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    if !resp.ok() {
        return Err(JsValue::from_str("Failed to fetch posts"));
    }

    resp.json().await.map_err(|e| JsValue::from_str(&e.to_string()))
}

pub async fn delete_post(id: i64) -> Result<(), JsValue> {
    let mut req = Request::delete(&format!("{BASE_URL}/posts/{id}"));

    if let Some(auth) = auth_header() {
        req = req.header("Authorization", &auth);
    }

    let resp = req.send().await.map_err(|e| JsValue::from_str(&e.to_string()))?;

    if !resp.ok() {
        let err: ErrorResponse = resp
            .json()
            .await
            .unwrap_or(ErrorResponse { error: "Failed to delete post".to_string() });
        return Err(JsValue::from_str(&err.error));
    }

    Ok(())
}
