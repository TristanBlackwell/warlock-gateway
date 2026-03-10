# Warlock Gateway

Lightweight VM registry for the [Warlock](https://github.com/TristanBlackwell/warlock) Firecracker control plane.

Maintains a centralized store of workers (hosts) and VMs (guests) enabling the proxying of requests between machines.

## Architecture

```
┌─────────────────────────────────────────┐
│         Warlock Gateway                 │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │      HTTP API (Axum)            │   │
│  │  - Worker registration          │   │
│  │  - VM registration              │   │
│  │  - VM location lookup           │   │
│  └──────────────┬──────────────────┘   │
│                 │                       │
│  ┌──────────────▼──────────────────┐   │
│  │    Registry (src/registry.rs)   │   │
│  │  - VM → Worker mapping          │   │
│  │  - Worker health tracking       │   │
│  └──────────────┬──────────────────┘   │
│                 │                       │
│  ┌──────────────▼──────────────────┐   │
│  │    SQLite Database              │   │
│  │  - workers table                │   │
│  │  - vms table                    │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## API

The gateway exposes a HTTP API. Endpoint saccept and return JSON.

### Worker Endpoints

**Register Worker**

```bash
POST /worker/register
Content-Type: application/json

{
    "worker_id": "worker-0",
    "ip_address": "10.10.0.4"
}
```

**Worker Heartbeat**

Called by an individual worker as a _liveliness_ check.

```bash
POST /worker/{worker_id}/heartbeat
```

### VM Endpoints

**Get VM Location**

Gets the IP of a VM and it's internal IP address. Used in the [warlock-infra](https://github.com/TristanBlackwell/warlock-infra) for the bastion server to proxy SSH to VMs within a private network.

```bash
GET /vm/{vm_id}/location

Response:
{
    "vm_id": "abc-123-def",
    "worker_ip": "10.10.0.4",
    "port": 2222,
    "status": "running"
}
```

**Register VM**
```bash
POST /vm/register
Content-Type: application/json

{
    "vm_id": "abc-123-def",
    "worker_id": "worker-0"
}
```

**Deregister VM**
```bash
DELETE /vm/{vm_id}
```

### Health Endpoint

**Gateway Health**
```bash
GET /internal/health

Response:
{
    "status": "ok",
    "workers": 3,
    "healthy_workers": 3,
    "vms": 42
}
```

## Development

### Prerequisites

- Rust v1.91+

Run the app:

```bash
cargo run
```

Or with Make:

```bash
make start
```

### Tests

```bash
cargo test
```

Or:

```bash
make test
```

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | SQLite database path | `/var/lib/warlock-gateway/registry.db` |
| `BIND_ADDR` | Server port | `0.0.0.0:8080` |
| `HEARTBEAT_TIMEOUT_SECS` | The maximum duration from the last heartbeat after which a worker is marked `unhealthy` | `60` |
| `HEALTH_CHECK_INTERVAL_SECS` | How frequently the health of workers is checked. | `30` |
| `RUST_LOG` | Tracing filter directive | `info` |

## Database Schema

The service uses SQLite with two tables:

**workers** - Tracks registered workers
- `id`: Worker identifier
- `ip_address`: Private VPC IP address
- `registered_at`: Registration timestamp
- `last_heartbeat`: Last heartbeat timestamp
- `status`: Worker status (healthy/unhealthy)

**vms** - Tracks VM to worker mappings
- `vm_id`: VM UUID
- `worker_id`: Worker hosting this VM
- `created_at`: Registration timestamp

## Health Monitoring

The gateway runs a background task that:
- Checks worker heartbeats every X seconds
- Marks workers as unhealthy if no heartbeat received within X seconds
- Prevents routing to VMs on unhealthy workers
