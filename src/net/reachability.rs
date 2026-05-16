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
    println!("[reachability] Checking {addr}");

    match timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    {
        // Connected successfully — service is listening.
        Ok(Ok(_stream)) => {
            println!("[reachability] {addr} — reachable, port open");
            ReachabilityResult::Reachable
        }
        // Connection attempt returned an error within the timeout.
        Ok(Err(e)) => {
            let kind = e.kind();
            if kind == ErrorKind::ConnectionRefused {
                // RST received → host is alive but port is closed.
                println!("[reachability] {addr} — host reachable, port closed (ConnectionRefused)");
                ReachabilityResult::HostReachablePortClosed
            } else {
                // Any other error (DNS failure, no route, network
                // unreachable, permission denied, …) → treat as
                // host unreachable.
                eprintln!("[reachability] {addr} — unreachable: {e} (kind={kind:?})");
                ReachabilityResult::HostUnreachable(e.to_string())
            }
        }
        // Timeout expired — host did not respond in time.
        Err(_) => {
            eprintln!("[reachability] {addr} — unreachable: connection timed out");
            ReachabilityResult::HostUnreachable("Connection timed out".to_string())
        }
    }
}
