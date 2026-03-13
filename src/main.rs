use anyhow::Result;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;

pub mod api;
pub mod config;
pub mod db;
pub mod health;
pub mod models;
pub mod registry;

use config::Config;
use db::Database;
use registry::Registry;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;
    info!("Starting warlock-gateway with config: {:?}", config);

    let db = Database::new(&config.database_url).await?;
    db.migrate().await?;

    let registry = Arc::new(Registry::new(db));

    health::spawn_health_monitor(
        registry.clone(),
        config.health_check_interval_secs,
        config.heartbeat_timeout_secs,
    );

    let app = Router::new()
        // VM endpoints
        .route("/vm", post(api::vm::create))
        .route("/vm/{vm_id}/location", get(api::vm::get_location))
        .route("/vm/register", post(api::vm::register))
        .route("/vm/{vm_id}", delete(api::vm::deregister))
        // Worker endpoints
        .route("/worker/register", post(api::worker::register))
        .route("/worker/{worker_id}/heartbeat", post(api::worker::heartbeat))
        // Health endpoint
        .route("/internal/health", get(api::health))
        // Shared state
        .with_state(registry)
        // Tracing middleware
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    info!("Gateway listening on {}", config.bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
