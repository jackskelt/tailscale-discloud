use axum::response::Json;

use serde::{Deserialize, Serialize};

use crate::net::utils::test_connection;

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub target_host: String,
    pub target_port: u16,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub log: String,
}

// ─── POST /api/test ─────────────────────────────────────────────────────

pub async fn test_endpoint(
    Json(payload): Json<TestConnectionRequest>,
) -> Json<TestConnectionResponse> {
    println!(
        "[POST /api/test] target={}:{}",
        payload.target_host, payload.target_port
    );

    let (success, log) = test_connection(&payload.target_host, payload.target_port).await;

    println!(
        "[POST /api/test] {}:{} -> {}",
        payload.target_host,
        payload.target_port,
        if success { "OK" } else { "FAIL" }
    );

    Json(TestConnectionResponse { success, log })
}
