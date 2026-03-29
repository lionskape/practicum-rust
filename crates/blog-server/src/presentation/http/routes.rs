use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::presentation::http::{auth_handlers, post_handlers, state::AppState};

pub fn create_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/register", post(auth_handlers::register))
        .route("/auth/login", post(auth_handlers::login))
        .route("/posts", post(post_handlers::create_post))
        .route("/posts", get(post_handlers::list_posts))
        .route("/posts/{id}", get(post_handlers::get_post))
        .route("/posts/{id}", put(post_handlers::update_post))
        .route("/posts/{id}", delete(post_handlers::delete_post));

    Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
