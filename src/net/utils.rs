use tokio::net::TcpListener;

/// Returns `true` when `host` resolves to a loopback address (localhost,
/// 127.x.x.x, ::1, 0.0.0.0). Used to detect self-loop configurations.
pub fn is_loopback_host(host: &str) -> bool {
    let h = host.trim().to_lowercase();
    if h == "localhost" || h == "::1" || h == "0.0.0.0" {
        return true;
    }
    // Match 127.0.0.0/8 (any 127.x.x.x)
    if let Ok(ip) = h.parse::<std::net::Ipv4Addr>() {
        return ip.octets()[0] == 127;
    }
    if let Ok(ip) = h.parse::<std::net::Ipv6Addr>() {
        return ip.is_loopback();
    }
    false
}

/// Check whether a given TCP port is available by attempting to bind to it.
pub async fn is_port_available(port: u16) -> bool {
    let is_avail = TcpListener::bind(("0.0.0.0", port)).await.is_ok();
    tracing::trace!(
        port = port,
        available = is_avail,
        "Port availability check result"
    );
    is_avail
}

/// Test connectivity to a host:port.
/// Returns `(success, combined_log)`.
pub async fn test_connection(target_host: &str, target_port: u16) -> (bool, String) {
    tracing::debug!(
        target_host = %target_host,
        target_port = target_port,
        "Running connection test"
    );

    let addr = format!("{target_host}:{target_port}");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    {
        Ok(Ok(_stream)) => {
            tracing::debug!(
                target_host = %target_host,
                target_port = target_port,
                "Connection test succeeded"
            );
            (true, "Connection succeeded".to_string())
        }
        Ok(Err(e)) => {
            let msg = format!("Connection to {target_host}:{target_port} failed: {e}");
            tracing::debug!(
                target_host = %target_host,
                target_port = target_port,
                log = %msg,
                "Connection test failed"
            );
            (false, msg)
        }
        Err(_) => {
            let msg = format!("Connection to {target_host}:{target_port} failed: connection timed out after 3 seconds");
            tracing::debug!(
                target_host = %target_host,
                target_port = target_port,
                log = %msg,
                "Connection test failed"
            );
            (false, msg)
        }
    }
}
