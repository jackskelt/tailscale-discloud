use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
/// Wraps the core Tunnel with computed fields and optional warnings.
#[derive(Debug, Clone, Serialize)]
pub struct TunnelResponse {
    #[serde(flatten)]
    pub tunnel: Tunnel,
    /// Connection URL for this tunnel, e.g. "tailscale:5432".
    /// Only present when the tunnel is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_url: Option<String>,
    /// Optional warning message (the tunnel was created/updated but
    /// something non-fatal was detected, such as the target port not
    /// responding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<ApiMessage>,
}

/// The response returned for listing tunnels.
/// Each item augments the core Tunnel with the connection URL.
#[derive(Debug, Clone, Serialize)]
pub struct TunnelListItem {
    #[serde(flatten)]
    pub tunnel: Tunnel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_url: Option<String>,
}

/// A structured message with an i18n key and interpolation parameters.
/// The frontend resolves the `id` via its i18n module and substitutes
/// the `params` placeholders.
#[derive(Debug, Clone, Serialize)]
pub struct ApiMessage {
    pub id: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, serde_json::Value>,
}

impl ApiMessage {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_params(id: impl Into<String>, params: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id: id.into(),
            params,
        }
    }
}

/// Structured API error returned to the frontend.
/// `error.id` is an i18n key; `error.params` carries any dynamic values
/// the frontend template needs for interpolation.
#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiMessage,
}

/// Result of a target reachability pre-flight check.
#[derive(Debug)]
pub enum ReachabilityResult {
    /// Both host and port are reachable — a service is listening.
    Reachable,
    /// Host responds (TCP RST / connection refused) but nothing is
    /// listening on the requested port.
    HostReachablePortClosed,
    /// The host could not be reached at all (timeout, DNS failure, etc.).
    HostUnreachable(String),
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

/// Response for GET /api/config
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub hostname: String,
    pub version: String,
}
