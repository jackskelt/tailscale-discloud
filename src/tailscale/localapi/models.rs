use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LocalNodeInfo {
    pub magicdns_hostname: String,
    pub dns_name: String,
    pub ipv4: String,
    pub ipv6: String,
}

#[derive(Debug, Deserialize)]
pub struct LocalApiStatus {
    #[serde(rename = "Self")]
    pub self_node: LocalApiSelf,
    #[serde(rename = "MagicDNSSuffix")]
    pub magicdns_suffix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LocalApiSelf {
    #[serde(rename = "DNSName")]
    pub dns_name: Option<String>,
    #[serde(rename = "TailscaleIPs")]
    pub tailscale_ips: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LocalStatus {
    #[serde(rename = "BackendState")]
    pub backend_state: String,
    #[serde(rename = "AuthURL")]
    pub auth_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Options {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_log_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_prefs: Option<Prefs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Prefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corp_dns: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub want_running: Option<bool>,

    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}
