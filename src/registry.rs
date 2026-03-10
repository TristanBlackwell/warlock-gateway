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

    /// Update worker heartbeat
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

        debug!("Heartbeat received from {}", worker_id);
        Ok(())
    }

    /// Register a VM
    pub async fn register_vm(&self, vm_id: &str, worker_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        
        sqlx::query!(
            "INSERT INTO vms (vm_id, worker_id, created_at) VALUES (?, ?, ?)",
            vm_id,
            worker_id,
            now
        )
        .execute(self.db.pool())
        .await
        .context("Failed to register VM")?;

        info!("VM registered: {} on {}", vm_id, worker_id);
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

    /// Deregister a VM
    pub async fn deregister_vm(&self, vm_id: &str) -> Result<()> {
        sqlx::query!("DELETE FROM vms WHERE vm_id = ?", vm_id)
            .execute(self.db.pool())
            .await
            .context("Failed to deregister VM")?;

        info!("VM deregistered: {}", vm_id);
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
}
