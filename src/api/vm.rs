use crate::models::{VmLocation, VmRegistration};
use crate::registry::Registry;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

/// Get VM location
pub async fn get_location(
    State(registry): State<Arc<Registry>>,
    Path(vm_id): Path<String>,
) -> Result<Json<VmLocation>, (StatusCode, String)> {
    let location = registry
        .get_vm_location(&vm_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to query VM location: {:#}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "VM not found".to_string()))?;

    Ok(Json(location))
}

/// Register a new VM
pub async fn register(
    State(registry): State<Arc<Registry>>,
    Json(req): Json<VmRegistration>,
) -> Result<StatusCode, (StatusCode, String)> {
    registry
        .register_vm(&req.vm_id, &req.worker_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to register VM: {:#}", e),
            )
        })?;

    Ok(StatusCode::CREATED)
}

/// Deregister a VM
pub async fn deregister(
    State(registry): State<Arc<Registry>>,
    Path(vm_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    registry.deregister_vm(&vm_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to deregister VM: {:#}", e),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}
