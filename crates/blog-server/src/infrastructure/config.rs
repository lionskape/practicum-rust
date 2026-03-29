use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub http_port: u16,
    pub grpc_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
            http_port: std::env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("HTTP_PORT must be a valid port number")?,
            grpc_port: std::env::var("GRPC_PORT")
                .unwrap_or_else(|_| "50051".to_string())
                .parse()
                .context("GRPC_PORT must be a valid port number")?,
        })
    }
}
