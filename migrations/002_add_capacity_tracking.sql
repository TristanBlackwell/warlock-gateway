-- Add capacity tracking columns to workers table
ALTER TABLE workers ADD COLUMN total_vcpus INTEGER;
ALTER TABLE workers ADD COLUMN total_memory_mb INTEGER;
ALTER TABLE workers ADD COLUMN allocatable_memory_mb INTEGER;

-- Add utilization tracking
ALTER TABLE workers ADD COLUMN allocated_vcpus INTEGER DEFAULT 0;
ALTER TABLE workers ADD COLUMN allocated_memory_mb INTEGER DEFAULT 0;
ALTER TABLE workers ADD COLUMN vm_count INTEGER DEFAULT 0;

-- Add metadata
ALTER TABLE workers ADD COLUMN version TEXT;
ALTER TABLE workers ADD COLUMN copy_strategy TEXT;

-- Add resource tracking to VMs
ALTER TABLE vms ADD COLUMN vcpus INTEGER;
ALTER TABLE vms ADD COLUMN memory_mb INTEGER;

-- Create index for capacity queries
CREATE INDEX idx_worker_capacity ON workers(status, allocated_vcpus, allocated_memory_mb);
