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
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vm {
    pub vm_id: String,
    pub worker_id: String,
    pub created_at: i64,
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
}
