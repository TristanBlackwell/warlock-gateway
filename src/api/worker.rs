use crate::models::WorkerRegistration;
use crate::registry::Registry;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

/// Register a new worker
pub async fn register(
    State(registry): State<Arc<Registry>>,
    Json(req): Json<WorkerRegistration>,
) -> Result<StatusCode, (StatusCode, String)> {
    registry
        .register_worker(&req.worker_id, &req.ip_address)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to register worker: {:#}", e),
            )
        })?;

    Ok(StatusCode::OK)
}

/// Worker heartbeat
pub async fn heartbeat(
    State(registry): State<Arc<Registry>>,
    Path(worker_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    registry.heartbeat(&worker_id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            format!("Worker not found: {:#}", e),
        )
    })?;

    Ok(StatusCode::OK)
}
