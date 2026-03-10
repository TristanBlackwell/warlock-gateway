pub mod vm;
pub mod worker;

use crate::registry::Registry;
use axum::{extract::State, Json};
use std::sync::Arc;

/// Health check endpoint
pub async fn health(State(registry): State<Arc<Registry>>) -> Json<crate::models::HealthStatus> {
    // If health_status fails, return a degraded status
    let status = registry
        .health_status()
        .await
        .unwrap_or(crate::models::HealthStatus {
            status: "degraded".to_string(),
            workers: 0,
            healthy_workers: 0,
            vms: 0,
        });
    
    Json(status)
}
