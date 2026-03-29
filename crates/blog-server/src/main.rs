use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;
use tonic::transport::Server as TonicServer;
use tracing_subscriber::EnvFilter;

mod application;
mod data;
mod domain;
mod infrastructure;
mod presentation;

use data::{post_repository::PgPostRepository, user_repository::PgUserRepository};
use infrastructure::{config::Config, database, jwt::JwtService};
use presentation::{
    grpc::{proto::blog_service_server::BlogServiceServer, service::BlogGrpcService},
    http::{routes::create_router, state::AppState},
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;
    let pool = database::create_pool(&config.database_url).await?;
    database::run_migrations(&pool).await?;

    let jwt = Arc::new(JwtService::new(&config.jwt_secret));
    let user_repo = PgUserRepository::new(pool.clone());
    let post_repo = PgPostRepository::new(pool.clone());

    let auth_service =
        Arc::new(application::auth_service::AuthService::new(user_repo, jwt.as_ref().clone()));
    let blog_service = Arc::new(application::blog_service::BlogService::new(post_repo));

    let app_state = AppState {
        auth_service: auth_service.clone(),
        blog_service: blog_service.clone(),
        jwt: jwt.clone(),
    };

    let http_addr = format!("0.0.0.0:{}", config.http_port);
    let grpc_addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;

    let router = create_router(app_state);
    let listener = TcpListener::bind(&http_addr).await?;
    tracing::info!("HTTP server listening on {http_addr}");

    let grpc_service = BlogGrpcService::new(auth_service, blog_service, jwt);
    tracing::info!("gRPC server listening on {grpc_addr}");

    tokio::select! {
        result = axum::serve(listener, router) => {
            result?;
        }
        result = TonicServer::builder()
            .add_service(BlogServiceServer::new(grpc_service))
            .serve(grpc_addr) => {
            result?;
        }
    }

    Ok(())
}
