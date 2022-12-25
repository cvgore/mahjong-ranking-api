mod app;
mod config;
mod db;
mod firebase;
mod games;
mod places;
mod users;
mod validate;
mod game_events;
mod players;
mod rankings;
mod ranks;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use crate::firebase::FirebaseTokenService;
use app::AppError;
use axum::handler::Handler;
use axum::response::IntoResponse;
use axum::routing::{any, get};
use axum::{Json, Router, Extension, http};
use hyper::StatusCode;
use serde_json::json;
use tower_http::compression::CompressionLayer;
use tracing::{debug, error, info};
use tower_http::cors::{Any, CorsLayer};

fn spawn_worker_thread(firebase: Arc<FirebaseTokenService>) -> tokio::task::JoinHandle<Infallible> {
    tokio::spawn(async move {
        loop {
            let ttl = firebase.update_auth_keys().await.unwrap_or_else(|e| {
                error!("failed to update firebase auth public keys: {}", e);
                std::time::Duration::from_secs(60)
            });
            debug!("will update firebase auth public keys in {:?}", ttl);

            tokio::time::sleep(ttl).await;
        }
    })
}

#[tokio::main]
async fn main() {
    let config = config::init_config();

    tracing_subscriber::fmt::init();
    info!("starting {} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let pool = db::init_db(&config).await;

    let firebase = Arc::new(FirebaseTokenService::new(
        config.firebase_project_id.clone(),
    ));

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    let app = Router::new()
        .route("/", any(no_way_in))
        .nest(
            "/v0",
            Router::new()
                .route("/", get(index))
                .merge(games::router())
                .merge(places::router())
                .merge(game_events::router())
                .merge(players::router())
                .merge(ranks::router())
                .merge(users::router())
                .merge(rankings::router())
                .layer(&cors),
        )
        .layer(&cors)
        .layer(CompressionLayer::new())
        .layer(Extension(pool))
        .layer(Extension(firebase.clone()))
        .fallback(not_found.layer(CompressionLayer::new()).layer(&cors).into_service());
    let addr = SocketAddr::from_str(&config.bind_interface).expect("malformed bind_interface str");

    let worker_thread = spawn_worker_thread(firebase);

    info!("listening on {}", addr);

    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await;
        
    worker_thread.abort();
    worker_thread.await.unwrap_err();

    server.unwrap()
}

async fn index() -> impl IntoResponse {
    Json(json!({
        "hello": "world"
    }))
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({"error": "not found"})))
}

async fn no_way_in() -> impl IntoResponse {
    AppError::Forbidden
}