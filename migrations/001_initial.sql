-- Workers table - tracks registered workers
CREATE TABLE workers (
    id TEXT PRIMARY KEY,              -- worker-0, worker-1, etc.
    ip_address TEXT NOT NULL,         -- Private VPC IP (e.g., 10.10.0.4)
    registered_at INTEGER NOT NULL,   -- Unix timestamp
    last_heartbeat INTEGER NOT NULL,  -- Unix timestamp
    status TEXT NOT NULL DEFAULT 'healthy'  -- 'healthy' or 'unhealthy'
);

-- VMs table - tracks which worker hosts each VM
CREATE TABLE vms (
    vm_id TEXT PRIMARY KEY,           -- UUID from Warlock
    worker_id TEXT NOT NULL,          -- Which worker hosts this VM
    created_at INTEGER NOT NULL,      -- Unix timestamp
    FOREIGN KEY (worker_id) REFERENCES workers(id) ON DELETE CASCADE
);

CREATE INDEX idx_worker_vms ON vms(worker_id);
CREATE INDEX idx_worker_status ON workers(status);
CREATE INDEX idx_last_heartbeat ON workers(last_heartbeat);
