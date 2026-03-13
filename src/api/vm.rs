use crate::models::{CreateVmRequest, CreateVmResponse, VmLocation, VmRegistration};
use crate::registry::Registry;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;

/// Create a new VM
pub async fn create(
    State(registry): State<Arc<Registry>>,
    Json(req): Json<CreateVmRequest>,
) -> Result<Json<CreateVmResponse>, (StatusCode, String)> {
    // Apply defaults (matches worker's behavior)
    let vcpus = req.vcpus.unwrap_or(1);
    let memory_mb = req.memory_mb.unwrap_or(128);

    info!("VM creation request: {}vCPU, {}MB", vcpus, memory_mb);

    let worker = registry
        .select_worker_for_vm(vcpus, memory_mb)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to query workers: {:#}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::INSUFFICIENT_STORAGE,
                format!(
                    "No workers available with sufficient capacity (need {}vCPU, {}MB)",
                    vcpus, memory_mb
                ),
            )
        })?;

    info!("Selected worker {} for VM placement", worker.id);

    // Proxy VM creation request to worker
    let client = reqwest::Client::new();
    let worker_url = format!("http://{}:3000/vm", worker.ip_address);

    let response = client
        .post(&worker_url)
        .json(&CreateVmRequest {
            vcpus: Some(vcpus),
            memory_mb: Some(memory_mb),
            ssh_keys: req.ssh_keys,
        })
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to contact worker: {:#}", e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            format!("Worker returned error: {}", error),
        ));
    }

    let vm: CreateVmResponse = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid worker response: {:#}", e),
        )
    })?;

    info!("VM created successfully: {} on {}", vm.id, worker.id);

    // NOTE: Worker will call POST /vm/register to register this VM
    // We don't need to manually register it here

    Ok(Json(vm))
}

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
        .register_vm(&req.vm_id, &req.worker_id, req.vcpus, req.memory_mb)
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
