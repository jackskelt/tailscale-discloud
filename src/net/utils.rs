use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::process::Command;

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

/// Test connectivity to a host:port using `nc -zvw3`.
/// Returns `(success, combined_log)`.
pub async fn test_connection(target_host: &str, target_port: u16) -> (bool, String) {
    tracing::debug!(
        target_host = %target_host,
        target_port = target_port,
        "Running netcat connection test"
    );

    let result = Command::new("nc")
        .arg("-zvw3")
        .arg(target_host)
        .arg(target_port.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match result {
        Ok(mut child) => {
            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();

            if let Some(ref mut stdout) = child.stdout {
                let _ = stdout.read_to_string(&mut stdout_buf).await;
            }
            if let Some(ref mut stderr) = child.stderr {
                let _ = stderr.read_to_string(&mut stderr_buf).await;
            }

            let status = child.wait().await;
            let success = status.map(|s| s.success()).unwrap_or(false);

            let mut log = String::new();
            if !stdout_buf.is_empty() {
                log.push_str(&stdout_buf);
            }
            if !stderr_buf.is_empty() {
                if !log.is_empty() {
                    log.push('\n');
                }
                log.push_str(&stderr_buf);
            }
            if log.is_empty() {
                log = if success {
                    "Connection succeeded".to_string()
                } else {
                    format!("Connection to {target_host}:{target_port} failed (no output)")
                };
            }

            if success {
                tracing::debug!(
                    target_host = %target_host,
                    target_port = target_port,
                    "Connection test succeeded"
                );
            } else {
                tracing::debug!(
                    target_host = %target_host,
                    target_port = target_port,
                    log = %log,
                    "Connection test failed"
                );
            }

            (success, log)
        }
        Err(e) => {
            let msg = format!("Failed to run nc: {e}");
            tracing::error!(error = %e, "Failed to execute nc subprocess");
            (false, msg)
        }
    }
}
