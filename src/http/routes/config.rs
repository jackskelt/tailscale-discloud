use axum::response::Json;

use serde::Serialize;

use crate::tailscale::localapi::get_local_node_info;

/// Response for GET /api/config
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    /// MagicDNS short hostname (without suffix).
    pub magicdns_hostname: String,
    /// MagicDNS fully-qualified DNS name.
    pub dns_name: String,
    /// Primary IPv4 address for the node (if any).
    pub ipv4: String,
    /// Primary IPv6 address for the node (if any).
    pub ipv6: String,
    pub version: String,
}

pub async fn get_config() -> Json<ConfigResponse> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    let (magicdns_hostname, dns_name, ipv4, ipv6) = match get_local_node_info().await {
        Ok(info) => (info.magicdns_hostname, info.dns_name, info.ipv4, info.ipv6),
        Err(e) => {
            eprintln!("[GET /api/config] {e} — MagicDNS unavailable");
            (String::new(), String::new(), String::new(), String::new())
        }
    };

    println!("[GET /api/config] magicdns={magicdns_hostname} dns_name={dns_name} ipv4={ipv4} ipv6={ipv6} version={version}");
    Json(ConfigResponse {
        magicdns_hostname,
        dns_name,
        ipv4,
        ipv6,
        version,
    })
}
