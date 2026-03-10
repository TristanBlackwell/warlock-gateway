use crate::registry::Registry;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error};

/// Spawn background task to monitor worker health
pub fn spawn_health_monitor(registry: Arc<Registry>, check_interval_secs: u64, timeout_secs: i64) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(check_interval_secs));
        
        loop {
            interval.tick().await;
            
            debug!("Running health check...");
            
            if let Err(e) = registry.check_worker_health(timeout_secs).await {
                error!("Health check failed: {:#}", e);
            }
        }
    });
}
