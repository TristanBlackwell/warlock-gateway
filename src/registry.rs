use crate::db::Database;
use crate::models::*;
use anyhow::{Context, Result};
use tracing::{debug, info};

pub struct Registry {
    db: Database,
}

impl Registry {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Register a new worker or update existing
    pub async fn register_worker(&self, worker_id: &str, ip_address: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        
        sqlx::query!(
            r#"
            INSERT INTO workers (id, ip_address, registered_at, last_heartbeat, status)
            VALUES (?, ?, ?, ?, 'healthy')
            ON CONFLICT(id) DO UPDATE SET
                ip_address = excluded.ip_address,
                last_heartbeat = excluded.last_heartbeat,
                status = 'healthy'
            "#,
            worker_id,
            ip_address,
            now,
            now
        )
        .execute(self.db.pool())
        .await
        .context("Failed to register worker")?;

        info!("Worker registered: {} @ {}", worker_id, ip_address);
        Ok(())
    }

    /// Update worker heartbeat and fetch capacity
    pub async fn heartbeat(&self, worker_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        
        let result = sqlx::query!(
            "UPDATE workers SET last_heartbeat = ?, status = 'healthy' WHERE id = ?",
            now,
            worker_id
        )
        .execute(self.db.pool())
        .await
        .context("Failed to update heartbeat")?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Worker not found: {}", worker_id);
        }

        // Fetch worker IP for capacity update
        let worker = sqlx::query!("SELECT ip_address FROM workers WHERE id = ?", worker_id)
            .fetch_one(self.db.pool())
            .await?;
        
        // Update capacity asynchronously (don't block heartbeat response)
        let db = self.db.clone();
        let worker_id_owned = worker_id.to_string();
        let ip_address = worker.ip_address;
        
        tokio::spawn(async move {
            let registry = Registry::new(db);
            if let Err(e) = registry.update_worker_capacity(&worker_id_owned, &ip_address).await {
                tracing::warn!("Failed to update capacity for {}: {:#}", worker_id_owned, e);
            }
        });

        debug!("Heartbeat received from {}", worker_id);
        Ok(())
    }

    /// Register a VM with resource tracking
    pub async fn register_vm(&self, vm_id: &str, worker_id: &str, vcpus: Option<u8>, memory_mb: Option<u32>) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        
        sqlx::query!(
            "INSERT INTO vms (vm_id, worker_id, created_at, vcpus, memory_mb) VALUES (?, ?, ?, ?, ?)",
            vm_id,
            worker_id,
            now,
            vcpus,
            memory_mb
        )
        .execute(self.db.pool())
        .await
        .context("Failed to register VM")?;

        // Update worker utilization (denormalized for performance)
        if let (Some(vcpus), Some(memory_mb)) = (vcpus, memory_mb) {
            sqlx::query!(
                r#"
                UPDATE workers 
                SET 
                    allocated_vcpus = COALESCE(allocated_vcpus, 0) + ?,
                    allocated_memory_mb = COALESCE(allocated_memory_mb, 0) + ?,
                    vm_count = COALESCE(vm_count, 0) + 1
                WHERE id = ?
                "#,
                vcpus,
                memory_mb,
                worker_id
            )
            .execute(self.db.pool())
            .await
            .context("Failed to update worker utilization")?;
        }

        info!("VM registered: {} on {} ({}vCPU, {}MB)", vm_id, worker_id, vcpus.unwrap_or(0), memory_mb.unwrap_or(0));
        Ok(())
    }

    /// Get VM location (for SSH proxy)
    pub async fn get_vm_location(&self, vm_id: &str) -> Result<Option<VmLocation>> {
        let result = sqlx::query!(
            r#"
            SELECT v.vm_id as "vm_id!", w.ip_address as "ip_address!", w.status as "status: WorkerStatus"
            FROM vms v
            JOIN workers w ON v.worker_id = w.id
            WHERE v.vm_id = ?
            "#,
            vm_id
        )
        .fetch_optional(self.db.pool())
        .await
        .context("Failed to query VM location")?;

        match result {
            Some(row) if row.status == WorkerStatus::Healthy => {
                Ok(Some(VmLocation {
                    vm_id: row.vm_id,
                    worker_ip: row.ip_address,
                    port: 2222,
                    status: "running".to_string(),
                }))
            }
            Some(_) => {
                // VM exists but worker is unhealthy
                Ok(None)
            }
            None => Ok(None),
        }
    }

    /// Deregister a VM and update worker capacity
    pub async fn deregister_vm(&self, vm_id: &str) -> Result<()> {
        // Get VM details and worker IP before deleting
        let vm_info = sqlx::query!(
            r#"
            SELECT 
                v.vm_id as "vm_id!", 
                v.worker_id as "worker_id!", 
                v.created_at as "created_at!", 
                v.vcpus, 
                v.memory_mb,
                w.ip_address as "ip_address!"
            FROM vms v
            JOIN workers w ON v.worker_id = w.id
            WHERE v.vm_id = ?
            "#,
            vm_id
        )
        .fetch_optional(self.db.pool())
        .await?;
        
        if let Some(vm_info) = vm_info {
            let worker_ip = vm_info.ip_address;
            
            // Call worker to delete the actual VM
            let client = reqwest::Client::new();
            let worker_url = format!("http://{}:3000/vm/{}", worker_ip, vm_id);
            
            match client
                .delete(&worker_url)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let error = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        tracing::warn!(
                            "Worker failed to delete VM {}: {} - {}",
                            vm_id,
                            status,
                            error
                        );
                    } else {
                        info!("VM {} deleted on worker {}", vm_id, worker_ip);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to contact worker {} to delete VM {}: {:#}",
                        worker_ip,
                        vm_id,
                        e
                    );
                }
            }
            
            // Delete VM from registry
            sqlx::query!("DELETE FROM vms WHERE vm_id = ?", vm_id)
                .execute(self.db.pool())
                .await
                .context("Failed to deregister VM")?;
            
            // Update worker utilization
            if let (Some(vcpus), Some(memory_mb)) = (vm_info.vcpus, vm_info.memory_mb) {
                sqlx::query!(
                    r#"
                    UPDATE workers 
                    SET 
                        allocated_vcpus = COALESCE(allocated_vcpus, 0) - ?,
                        allocated_memory_mb = COALESCE(allocated_memory_mb, 0) - ?,
                        vm_count = COALESCE(vm_count, 0) - 1
                    WHERE id = ?
                    "#,
                    vcpus,
                    memory_mb,
                    vm_info.worker_id
                )
                .execute(self.db.pool())
                .await
                .context("Failed to update worker utilization")?;
            }
            
            info!("VM deregistered: {}", vm_id);
        }
        
        Ok(())
    }

    /// Get health status
    pub async fn health_status(&self) -> Result<HealthStatus> {
        let workers = sqlx::query_scalar!("SELECT COUNT(*) FROM workers")
            .fetch_one(self.db.pool())
            .await?;

        let healthy_workers = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM workers WHERE status = 'healthy'"
        )
        .fetch_one(self.db.pool())
        .await?;

        let vms = sqlx::query_scalar!("SELECT COUNT(*) FROM vms")
            .fetch_one(self.db.pool())
            .await?;

        Ok(HealthStatus {
            status: "ok".to_string(),
            workers,
            healthy_workers,
            vms,
        })
    }

    /// Mark stale workers as unhealthy
    pub async fn check_worker_health(&self, timeout_secs: i64) -> Result<()> {
        let cutoff = chrono::Utc::now().timestamp() - timeout_secs;
        
        let result = sqlx::query!(
            "UPDATE workers SET status = 'unhealthy' WHERE last_heartbeat < ? AND status = 'healthy'",
            cutoff
        )
        .execute(self.db.pool())
        .await?;

        if result.rows_affected() > 0 {
            info!("Marked {} workers as unhealthy", result.rows_affected());
        }

        Ok(())
    }

    /// Fetch and update worker capacity from /internal/ready endpoint
    pub async fn update_worker_capacity(&self, worker_id: &str, ip_address: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("http://{}:3000/internal/ready", ip_address);
        
        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .context("Failed to fetch worker ready status")?;
        
        let ready: WorkerReadyResponse = response
            .json()
            .await
            .context("Failed to parse ready response")?;
        
        // Update database with capacity info
        let capacity = ready.capacity.unwrap_or(WorkerCapacity {
            total_memory_mb: 0,
            allocatable_memory_mb: 0,
            total_vcpus: 0,
        });
        let utilization = ready.vms.unwrap_or(WorkerUtilization {
            count: 0,
            allocated_vcpus: 0,
            allocated_memory_mb: 0,
        });
        
        // Convert to i64 for SQLite
        let total_vcpus = capacity.total_vcpus as i64;
        let total_memory_mb = capacity.total_memory_mb as i64;
        let allocatable_memory_mb = capacity.allocatable_memory_mb as i64;
        let allocated_vcpus = utilization.allocated_vcpus as i64;
        let allocated_memory_mb = utilization.allocated_memory_mb as i64;
        let vm_count = utilization.count as i64;
        
        sqlx::query!(
            r#"
            UPDATE workers 
            SET 
                total_vcpus = ?,
                total_memory_mb = ?,
                allocatable_memory_mb = ?,
                allocated_vcpus = ?,
                allocated_memory_mb = ?,
                vm_count = ?,
                version = ?,
                copy_strategy = ?
            WHERE id = ?
            "#,
            total_vcpus,
            total_memory_mb,
            allocatable_memory_mb,
            allocated_vcpus,
            allocated_memory_mb,
            vm_count,
            ready.version,
            ready.copy_strategy,
            worker_id
        )
        .execute(self.db.pool())
        .await
        .context("Failed to update worker capacity")?;
        
        debug!(
            "Updated capacity for {}: {} vCPUs, {} MB (allocated: {} vCPUs, {} MB)",
            worker_id,
            capacity.total_vcpus,
            capacity.allocatable_memory_mb,
            utilization.allocated_vcpus,
            utilization.allocated_memory_mb
        );
        
        Ok(())
    }

    /// Select the best worker for VM placement using best-fit algorithm
    pub async fn select_worker_for_vm(&self, vcpus: u8, memory_mb: u32) -> Result<Option<WorkerWithCapacity>> {
        let vcpus_i64 = vcpus as i64;
        let memory_mb_i64 = memory_mb as i64;
        
        let worker = sqlx::query!(
            r#"
            SELECT 
                id as "id!",
                ip_address as "ip_address!",
                status as "status!: WorkerStatus",
                total_vcpus,
                allocatable_memory_mb,
                allocated_vcpus,
                allocated_memory_mb,
                vm_count
            FROM workers
            WHERE 
                status = 'healthy'
                AND total_vcpus IS NOT NULL
                AND allocatable_memory_mb IS NOT NULL
                AND (total_vcpus - COALESCE(allocated_vcpus, 0)) >= ?
                AND (allocatable_memory_mb - COALESCE(allocated_memory_mb, 0)) >= ?
            ORDER BY 
                (allocatable_memory_mb - COALESCE(allocated_memory_mb, 0)) ASC,
                (total_vcpus - COALESCE(allocated_vcpus, 0)) ASC
            LIMIT 1
            "#,
            vcpus_i64,
            memory_mb_i64
        )
        .fetch_optional(self.db.pool())
        .await
        .context("Failed to query available workers")?;
        
        Ok(worker.map(|w| WorkerWithCapacity {
            id: w.id,
            ip_address: w.ip_address,
            status: w.status,
            total_vcpus: w.total_vcpus,
            allocatable_memory_mb: w.allocatable_memory_mb,
            allocated_vcpus: w.allocated_vcpus,
            allocated_memory_mb: w.allocated_memory_mb,
            vm_count: w.vm_count,
        }))
    }
}
