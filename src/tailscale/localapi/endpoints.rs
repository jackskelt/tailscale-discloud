use crate::tailscale::localapi::client::request;
use crate::tailscale::localapi::models::{
    LocalApiStatus, LocalNodeInfo, LocalStatus, Options, Prefs,
};

/// Fetch the current node info from the Tailscale LocalAPI.
pub async fn get_local_node_info() -> Result<LocalNodeInfo, String> {
    let body = request(
        hyper::Method::GET,
        "/localapi/v0/status",
        bytes::Bytes::new(),
    )
    .await?;

    let parsed: LocalApiStatus = serde_json::from_slice(&body).map_err(|e| {
        let msg = format!("[localapi] JSON parse failed: {e}");
        tracing::error!(error = %e, "Failed to parse LocalAPI JSON response");
        msg
    })?;

    let dns_name =
        trim_trailing_dot(parsed.self_node.dns_name.as_deref().unwrap_or("").trim()).to_string();
    let magicdns_hostname =
        derive_magicdns_hostname(&dns_name, parsed.magicdns_suffix.as_deref().unwrap_or(""));

    let (ipv4, ipv6) = split_ips(&parsed.self_node.tailscale_ips);

    tracing::debug!(
        magicdns_hostname = %magicdns_hostname,
        dns_name = %dns_name,
        ipv4 = %ipv4,
        ipv6 = %ipv6,
        "Successfully parsed LocalNodeInfo from LocalAPI"
    );

    Ok(LocalNodeInfo {
        magicdns_hostname,
        dns_name,
        ipv4,
        ipv6,
    })
}

/// Fetch the raw LocalStatus from the Tailscale LocalAPI to check connection/login state.
pub async fn get_local_status() -> Result<LocalStatus, String> {
    let body = request(
        hyper::Method::GET,
        "/localapi/v0/status",
        bytes::Bytes::new(),
    )
    .await?;

    let parsed: LocalStatus =
        serde_json::from_slice(&body).map_err(|e| format!("[localapi] JSON parse failed: {e}"))?;

    Ok(parsed)
}

/// Fetch the current Preferences from the Tailscale LocalAPI.
pub async fn get_prefs() -> Result<Prefs, String> {
    let body = request(
        hyper::Method::GET,
        "/localapi/v0/prefs",
        bytes::Bytes::new(),
    )
    .await?;

    let parsed: Prefs = serde_json::from_slice(&body)
        .map_err(|e| format!("[localapi] get_prefs JSON parse failed: {e}"))?;

    Ok(parsed)
}

/// Start/configure the Tailscale backend with the provided options.
pub async fn start(opts: Options) -> Result<(), String> {
    let body_bytes = serde_json::to_vec(&opts)
        .map_err(|e| format!("[localapi] start failed to serialize options: {e}"))?;

    request(
        hyper::Method::POST,
        "/localapi/v0/start",
        bytes::Bytes::from(body_bytes),
    )
    .await?;

    Ok(())
}

/// Login interactive
pub async fn login_interactive() -> Result<(), String> {
    request(
        hyper::Method::POST,
        "/localapi/v0/login-interactive",
        bytes::Bytes::new(),
    )
    .await?;

    Ok(())
}

fn trim_trailing_dot(s: &str) -> &str {
    s.strip_suffix('.').unwrap_or(s)
}

fn derive_magicdns_hostname(dns_name: &str, magicdns_suffix: &str) -> String {
    let dns = trim_trailing_dot(dns_name);
    let suffix = trim_trailing_dot(magicdns_suffix);
    if dns.is_empty() || suffix.is_empty() {
        return String::new();
    }
    let needle = format!(".{suffix}");
    if let Some(stripped) = dns.strip_suffix(&needle) {
        stripped.to_string()
    } else {
        String::new()
    }
}

fn split_ips(ips: &[String]) -> (String, String) {
    let mut ipv4 = String::new();
    let mut ipv6 = String::new();

    for ip in ips {
        if let Ok(parsed) = ip.parse::<std::net::IpAddr>() {
            match parsed {
                std::net::IpAddr::V4(_) if ipv4.is_empty() => ipv4 = ip.clone(),
                std::net::IpAddr::V6(_) if ipv6.is_empty() => ipv6 = ip.clone(),
                _ => {}
            }
        }
    }

    (ipv4, ipv6)
}
