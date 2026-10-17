use axum::{
    Router, middleware,
    routing::{get, post},
};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{str::FromStr, sync::Arc};
use tower_http::services::ServeDir;

use crate::{config::Config, state::AppState};

mod auth;
mod config;
mod error;
mod models;
mod routes;
mod services;
mod state;
mod time_range;

fn register_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let routes = Router::new()
        .route("/beat", post(routes::beat))
        // Only applies to the routes above; anything added after this stays public
        .route_layer(middleware::from_fn_with_state(state, auth::require_api_key))
        .route("/status/{device_id}", get(routes::status))
        .route("/summary/{device_id}", get(routes::summary))
        .route("/period/{device_id}", get(routes::periods))
        .route("/health", get(|| async { "OK" }))
        // Paths that don't match a route above are served from the static/ folder:
        // `/` serves static/index.html, `/app.js` serves static/app.js, and missing files 404
        .fallback_service(ServeDir::new("static"));

    routes
}

async fn setup_database(database_url: &str) -> Result<sqlx::SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);

    SqlitePool::connect_with(options).await
}

#[tokio::main]
async fn main() {
    // Load dotenv
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,web=debug".into()),
        )
        .init();

    // Server setup
    let config = Config::from_env().unwrap();

    // Setup the database
    let db = setup_database(&config.database_url).await.unwrap();

    // run migrations
    sqlx::migrate!("./migrations").run(&db).await.unwrap();

    let state = Arc::new(state::AppState {
        db,
        config: config.clone(),
    });

    // Register routes
    let app = Router::new()
        .merge(register_routes(state.clone()))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.bind_address)
        .await
        .unwrap();

    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
