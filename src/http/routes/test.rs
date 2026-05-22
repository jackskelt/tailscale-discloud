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
    let (success, log) = test_connection(&payload.target_host, payload.target_port).await;

    tracing::debug!(
        target_host = %payload.target_host,
        target_port = payload.target_port,
        success = success,
        "Completed test connection endpoint check"
    );

    Json(TestConnectionResponse { success, log })
}
