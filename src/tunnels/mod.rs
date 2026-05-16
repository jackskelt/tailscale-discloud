use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::http::errors::ApiMessage;

pub mod service;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub local_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub enabled: bool,
    #[serde(skip_serializing)]
    #[serde(default)]
    pub pid: Option<u32>,
    /// Named `warning_id` to avoid a serde flatten collision with
    /// `TunnelResponse.warning` (which carries the full `ApiMessage`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning_id: Option<String>,
}

/// The response returned for a single tunnel (create / update / toggle).
/// Wraps the core Tunnel with optional warnings.
#[derive(Debug, Clone, Serialize)]
pub struct TunnelResponse {
    #[serde(flatten)]
    pub tunnel: Tunnel,
    /// Optional warning message (the tunnel was created/updated but
    /// something non-fatal was detected, such as the target port not
    /// responding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<ApiMessage>,
}

/// The response returned for listing tunnels.
#[derive(Debug, Clone, Serialize)]
pub struct TunnelListItem {
    #[serde(flatten)]
    pub tunnel: Tunnel,
}

// ─── Request payloads ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTunnelRequest {
    pub name: String,
    pub local_port: u16,
    pub target_host: String,
    pub target_port: u16,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateTunnelRequest {
    pub name: Option<String>,
    pub local_port: Option<u16>,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
    pub enabled: Option<bool>,
}

pub type SharedState = Arc<RwLock<Vec<Tunnel>>>;
