use serde::Deserialize;

const TAILSCALED_SOCKET_ENV: &str = "TAILSCALE_SOCKET";
const DEFAULT_TAILSCALED_SOCKET: &str = "/var/run/tailscale/tailscaled.sock";
const LOCALAPI_HOST_HEADER: &str = "local-tailscaled.sock";

#[derive(Debug, Clone)]
pub struct LocalNodeInfo {
    pub magicdns_hostname: String,
    pub dns_name: String,
    pub ipv4: String,
    pub ipv6: String,
}

#[derive(Debug, Deserialize)]
struct LocalApiStatus {
    #[serde(rename = "Self")]
    self_node: LocalApiSelf,
    #[serde(rename = "MagicDNSSuffix")]
    magicdns_suffix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalApiSelf {
    #[serde(rename = "DNSName")]
    dns_name: Option<String>,
    #[serde(rename = "TailscaleIPs")]
    tailscale_ips: Vec<String>,
}

fn tailscaled_socket_path() -> String {
    std::env::var(TAILSCALED_SOCKET_ENV).unwrap_or_else(|_| DEFAULT_TAILSCALED_SOCKET.to_string())
}

/// Fetch the current node info from the Tailscale LocalAPI.
pub async fn get_local_node_info() -> Result<LocalNodeInfo, String> {
    use hyper::{Body, Client, Request};
    use hyperlocal::{UnixClientExt, Uri};

    let socket_path = tailscaled_socket_path();
    let client: Client<_, Body> = Client::unix();
    let uri: hyper::Uri = Uri::new(socket_path, "/localapi/v0/status").into();

    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("Host", LOCALAPI_HOST_HEADER)
        .body(Body::empty())
        .map_err(|e| format!("[localapi] Request build failed: {e}"))?;

    let res = client
        .request(req)
        .await
        .map_err(|e| format!("[localapi] Request failed: {e}"))?;

    let status = res.status();
    let body = hyper::body::to_bytes(res.into_body())
        .await
        .map_err(|e| format!("[localapi] Read body failed: {e}"))?;

    if !status.is_success() {
        let body_str = String::from_utf8_lossy(&body);
        return Err(format!(
            "[localapi] HTTP {status} while reading status: {body_str}"
        ));
    }

    let parsed: LocalApiStatus =
        serde_json::from_slice(&body).map_err(|e| format!("[localapi] JSON parse failed: {e}"))?;

    let dns_name =
        trim_trailing_dot(parsed.self_node.dns_name.as_deref().unwrap_or("").trim()).to_string();
    let magicdns_hostname =
        derive_magicdns_hostname(&dns_name, parsed.magicdns_suffix.as_deref().unwrap_or(""));

    let (ipv4, ipv6) = split_ips(&parsed.self_node.tailscale_ips);

    Ok(LocalNodeInfo {
        magicdns_hostname,
        dns_name,
        ipv4,
        ipv6,
    })
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
