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

/// Pre-flight check: attempt a TCP connection to `host:port` with a short
/// timeout to determine whether the target is reachable and whether the
/// port has a service listening.
///
/// - `Reachable` — TCP handshake succeeded; a service is listening.
/// - `HostReachablePortClosed` — host responded with RST (connection
///   refused); host is alive but nothing listens on that port.
/// - `HostUnreachable` — timeout, DNS failure, no route, or any other
///   error that indicates the host itself cannot be contacted.
pub async fn check_target_reachability(host: &str, port: u16) -> ReachabilityResult {
    use std::io::ErrorKind;
    use tokio::time::{timeout, Duration};

    let addr = format!("{host}:{port}");
    tracing::debug!(address = %addr, "Initiating pre-flight TCP reachability check");

    match timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    {
        // Connected successfully — service is listening.
        Ok(Ok(_stream)) => {
            tracing::debug!(address = %addr, "Target is reachable and port is open");
            ReachabilityResult::Reachable
        }
        // Connection attempt returned an error within the timeout.
        Ok(Err(e)) => {
            let kind = e.kind();
            if kind == ErrorKind::ConnectionRefused {
                // RST received → host is alive but port is closed.
                tracing::debug!(address = %addr, "Host is reachable but port is closed (ConnectionRefused)");
                ReachabilityResult::HostReachablePortClosed
            } else {
                // Any other error (DNS failure, no route, network
                // unreachable, permission denied, …) → treat as
                // host unreachable.
                tracing::debug!(address = %addr, error = %e, error_kind = ?kind, "Target is unreachable");
                ReachabilityResult::HostUnreachable(e.to_string())
            }
        }
        // Timeout expired — host did not respond in time.
        Err(_) => {
            tracing::debug!(address = %addr, "Target connection attempt timed out after 3 seconds");
            ReachabilityResult::HostUnreachable("Connection timed out".to_string())
        }
    }
}
