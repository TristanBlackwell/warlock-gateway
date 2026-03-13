use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum WorkerStatus {
    Healthy,
    Unhealthy,
}

impl std::fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerStatus::Healthy => write!(f, "healthy"),
            WorkerStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Worker {
    pub id: String,
    pub ip_address: String,
    pub registered_at: i64,
    pub last_heartbeat: i64,
    pub status: WorkerStatus,

    // Capacity tracking
    pub total_vcpus: Option<i64>,
    pub total_memory_mb: Option<i64>,
    pub allocatable_memory_mb: Option<i64>,
    pub allocated_vcpus: Option<i64>,
    pub allocated_memory_mb: Option<i64>,
    pub vm_count: Option<i64>,
    pub version: Option<String>,
    pub copy_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vm {
    pub vm_id: String,
    pub worker_id: String,
    pub created_at: i64,

    // Resource tracking
    pub vcpus: Option<i64>,
    pub memory_mb: Option<i64>,
}

/// VM location response (for SSH proxy)
#[derive(Debug, Serialize, Deserialize)]
pub struct VmLocation {
    pub vm_id: String,
    pub worker_ip: String,
    pub port: u16,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub workers: i64,
    pub healthy_workers: i64,
    pub vms: i64,
}

#[derive(Debug, Deserialize)]
pub struct WorkerRegistration {
    pub worker_id: String,
    pub ip_address: String,
}

#[derive(Debug, Deserialize)]
pub struct VmRegistration {
    pub vm_id: String,
    pub worker_id: String,

    // Resource tracking
    pub vcpus: Option<u8>,
    pub memory_mb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCapacity {
    pub total_memory_mb: u64,
    pub allocatable_memory_mb: u64,
    pub total_vcpus: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerUtilization {
    pub count: usize,
    pub allocated_vcpus: u8,
    pub allocated_memory_mb: u64,
}

#[derive(Debug, Deserialize)]
pub struct WorkerReadyResponse {
    pub status: String,
    pub version: String,
    pub capacity: Option<WorkerCapacity>,
    pub vms: Option<WorkerUtilization>,
    pub copy_strategy: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVmRequest {
    pub vcpus: Option<u8>,
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub ssh_keys: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVmResponse {
    pub id: String,
    pub vcpus: u8,
    pub memory_mb: u32,
    pub state: String,
    pub vmm_version: String,
    pub guest_ip: String,
}

#[derive(Debug, Clone)]
pub struct WorkerWithCapacity {
    pub id: String,
    pub ip_address: String,
    pub status: WorkerStatus,
    pub total_vcpus: Option<i64>,
    pub allocatable_memory_mb: Option<i64>,
    pub allocated_vcpus: Option<i64>,
    pub allocated_memory_mb: Option<i64>,
    pub vm_count: Option<i64>,
}
