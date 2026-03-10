use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use warlock_gateway::*;

async fn setup_test_server() -> (String, TempDir) {
    // Create temporary directory for test database
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}", db_path.display());

    // Initialize database
    let db = db::Database::new(&db_url).await.unwrap();
    db.migrate().await.unwrap();

    // Initialize registry
    let registry = Arc::new(registry::Registry::new(db));

    // Build router
    let app = axum::Router::new()
        .route("/vm/{vm_id}/location", axum::routing::get(api::vm::get_location))
        .route("/vm/register", axum::routing::post(api::vm::register))
        .route("/vm/{vm_id}", axum::routing::delete(api::vm::deregister))
        .route("/worker/register", axum::routing::post(api::worker::register))
        .route("/worker/{worker_id}/heartbeat", axum::routing::post(api::worker::heartbeat))
        .route("/internal/health", axum::routing::get(api::health))
        .with_state(registry);

    // Start server on random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    (base_url, temp_dir)
}

#[tokio::test]
async fn test_full_lifecycle() {
    let (base_url, _temp_dir) = setup_test_server().await;
    let client = Client::new();

    // Register worker
    let resp = client
        .post(&format!("{}/worker/register", base_url))
        .json(&json!({
            "worker_id": "test-worker",
            "ip_address": "10.0.0.1"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Register VM
    let resp = client
        .post(&format!("{}/vm/register", base_url))
        .json(&json!({
            "vm_id": "test-vm",
            "worker_id": "test-worker"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Get VM location
    let resp = client
        .get(&format!("{}/vm/test-vm/location", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["vm_id"], "test-vm");
    assert_eq!(body["worker_ip"], "10.0.0.1");
    assert_eq!(body["port"], 2222);
    assert_eq!(body["status"], "running");

    // Deregister VM
    let resp = client
        .delete(&format!("{}/vm/test-vm", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Verify VM is gone
    let resp = client
        .get(&format!("{}/vm/test-vm/location", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_health_endpoint() {
    let (base_url, _temp_dir) = setup_test_server().await;
    let client = Client::new();

    let resp = client
        .get(&format!("{}/internal/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["workers"], 0);
    assert_eq!(body["healthy_workers"], 0);
    assert_eq!(body["vms"], 0);
}

#[tokio::test]
async fn test_worker_heartbeat() {
    let (base_url, _temp_dir) = setup_test_server().await;
    let client = Client::new();

    // Register worker
    client
        .post(&format!("{}/worker/register", base_url))
        .json(&json!({
            "worker_id": "test-worker",
            "ip_address": "10.0.0.1"
        }))
        .send()
        .await
        .unwrap();

    // Send heartbeat
    let resp = client
        .post(&format!("{}/worker/test-worker/heartbeat", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Heartbeat for non-existent worker should fail
    let resp = client
        .post(&format!("{}/worker/non-existent/heartbeat", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_vm_not_found() {
    let (base_url, _temp_dir) = setup_test_server().await;
    let client = Client::new();

    let resp = client
        .get(&format!("{}/vm/non-existent-vm/location", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
