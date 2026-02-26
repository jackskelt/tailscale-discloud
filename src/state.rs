use std::sync::Arc;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::models::{ReachabilityResult, Tunnel};

fn tunnels_path() -> String {
    std::env::var("TUNNELS_PATH").unwrap_or_else(|_| "./tunnels.json".to_string())
}

pub type SharedState = Arc<RwLock<Vec<Tunnel>>>;

/// Return the Tailscale hostname from the environment variable
/// `TAILSCALE_HOSTNAME`, falling back to `"tailscale-discloud"`.
pub fn get_hostname() -> String {
    std::env::var("TAILSCALE_HOSTNAME").unwrap_or_else(|_| "tailscale-discloud".to_string())
}

/// Build a connection URL for a tunnel: `<hostname>:<local_port>`.
/// Returns `Some(url)` when the tunnel is enabled, `None` otherwise.
pub fn connection_url_for(tunnel: &Tunnel) -> Option<String> {
    if tunnel.enabled {
        Some(format!("{}:{}", get_hostname(), tunnel.local_port))
    } else {
        None
    }
}

/// Returns `true` when `host` resolves to a loopback address (localhost,
/// 127.x.x.x, ::1, 0.0.0.0).  Used to detect self-loop configurations
/// where socat would forward traffic back to itself.
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

/// Load tunnels from the JSON persistence file.
/// Returns an empty vec if the file doesn't exist or is invalid.
pub async fn load_tunnels() -> Vec<Tunnel> {
    let path = tunnels_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => {
            let tunnels: Vec<Tunnel> = serde_json::from_str(&contents).unwrap_or_default();
            println!("[state] Loaded {} tunnel(s) from {}", tunnels.len(), path);
            tunnels
        }
        Err(e) => {
            eprintln!("[state] Could not read {path}: {e} — starting with empty list");
            Vec::new()
        }
    }
}

/// Persist the current tunnel list to disk.
/// PID fields are skipped during serialization automatically.
pub async fn save_tunnels(tunnels: &[Tunnel]) -> Result<(), String> {
    let path = tunnels_path();
    let json = serde_json::to_string_pretty(tunnels).map_err(|e| {
        let msg = format!("[state] JSON serialization error: {e}");
        eprintln!("{msg}");
        msg
    })?;

    tokio::fs::write(&path, json).await.map_err(|e| {
        let msg = format!("[state] Failed to write {path}: {e}");
        eprintln!("{msg}");
        msg
    })?;

    println!("[state] Persisted {} tunnel(s) to {}", tunnels.len(), path);
    Ok(())
}

/// Check whether a given TCP port is available by attempting to bind to it.
pub async fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).await.is_ok()
}

/// Spawn a `socat` process that forwards `local_port` -> `target_host:target_port`.
///
/// After spawning, waits briefly and verifies the process is still alive.
/// If socat exits immediately (bad args, port conflict, etc.) the stderr
/// output is captured and returned as an error — no zombie / orphan is left.
pub async fn spawn_socat(
    local_port: u16,
    target_host: &str,
    target_port: u16,
) -> Result<u32, String> {
    let listen_arg = format!("TCP-LISTEN:{local_port},fork,reuseaddr");
    let connect_arg = format!("TCP:{target_host}:{target_port}");

    println!("[socat] Spawning: socat {listen_arg} {connect_arg}");

    use std::os::unix::process::CommandExt;

    let mut std_cmd = std::process::Command::new("socat");
    std_cmd.process_group(0);

    let mut child = Command::from(std_cmd)
        .arg(&listen_arg)
        .arg(&connect_arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(false)
        .spawn()
        .map_err(|e| {
            let msg = format!("[socat] Failed to spawn socat: {e}");
            eprintln!("{msg}");
            msg
        })?;

    let pid = child
        .id()
        .ok_or_else(|| "[socat] Failed to obtain socat PID".to_string())?;

    println!("[socat] Spawned with PID {pid} (and PGID {pid}), verifying it stays alive...");

    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    match child.try_wait() {
        Ok(Some(status)) => {
            let mut stderr_output = String::new();
            if let Some(ref mut stderr) = child.stderr {
                let _ = stderr.read_to_string(&mut stderr_output).await;
            }
            let stderr_output = stderr_output.trim().to_string();
            let detail = if stderr_output.is_empty() {
                format!("exit {status}")
            } else {
                format!("exit {status}: {stderr_output}")
            };
            let msg = format!("[socat] PID {pid} exited immediately — {detail}");
            eprintln!("{msg}");
            Err(msg)
        }
        Ok(_) => {
            println!("[socat] PID {pid} is alive and listening on :{local_port}");

            tokio::spawn(async move {
                match child.wait().await {
                    Ok(status) => eprintln!("[socat] PID {pid} exited with {status}"),
                    Err(e) => eprintln!("[socat] PID {pid} wait error: {e}"),
                }
            });

            Ok(pid)
        }
        Err(e) => {
            let _ = Command::new("kill")
                .arg("-9")
                .arg(format!("-{}", pid))
                .output()
                .await;
            let msg = format!("[socat] Failed to check PID {pid} status: {e}");
            eprintln!("{msg}");
            Err(msg)
        }
    }
}

/// Kill a socat process by PID using `kill -9`.
/// Kill a socat process AND all its children by targeting the Process Group (PGID).
pub async fn kill_socat(pid: u32) -> Result<(), String> {
    println!("[socat] Killing Process Group for PID {pid}");

    let status = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!("kill -9 -{}", pid))
        .status()
        .await
        .map_err(|e| {
            let msg = format!("[socat] Failed to execute shell kill for PGID {pid}: {e}");
            eprintln!("{msg}");
            msg
        })?;

    if status.success() {
        println!("[socat] Process group {pid} killed successfully");
    } else {
        eprintln!("[socat] kill command exited with {status} for PGID {pid} (may already be dead)");
    }

    Ok(())
}

/// Test connectivity to a host:port using `nc -zvw3`.
/// Returns `(success, combined_log)`.
pub async fn test_connection(target_host: &str, target_port: u16) -> (bool, String) {
    println!("[test] Testing connection to {target_host}:{target_port}");

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
                println!("[test] {target_host}:{target_port} — OK");
            } else {
                eprintln!("[test] {target_host}:{target_port} — FAILED: {log}");
            }

            (success, log)
        }
        Err(e) => {
            let msg = format!("Failed to run nc: {e}");
            eprintln!("[test] {msg}");
            (false, msg)
        }
    }
}

/// Restore tunnels on boot: for each enabled tunnel whose port is free,
/// attempt **once** to spawn socat.  If the spawn fails the tunnel is
/// marked `enabled = false` so we never retry in an infinite loop.
pub async fn restore_tunnels(state: &SharedState) {
    let mut tunnels = state.write().await;
    let total = tunnels.len();
    let mut restored = 0u32;
    let mut failed = 0u32;

    println!("[boot] Restoring {total} tunnel(s)...");

    for tunnel in tunnels.iter_mut() {
        if !tunnel.enabled {
            tunnel.pid = None;
            continue;
        }

        if !is_port_available(tunnel.local_port).await {
            eprintln!(
                "[boot] Port {} is already in use — disabling tunnel '{}'",
                tunnel.local_port, tunnel.name
            );
            tunnel.enabled = false;
            tunnel.pid = None;
            failed += 1;
            continue;
        }

        match spawn_socat(tunnel.local_port, &tunnel.target_host, tunnel.target_port).await {
            Ok(pid) => {
                println!(
                    "[boot] Restored '{}' (:{} -> {}:{}) PID {pid}",
                    tunnel.name, tunnel.local_port, tunnel.target_host, tunnel.target_port
                );
                tunnel.pid = Some(pid);
                restored += 1;
            }
            Err(e) => {
                eprintln!(
                    "[boot] Failed to restore '{}': {e} — marking as disabled",
                    tunnel.name
                );
                tunnel.enabled = false;
                tunnel.pid = None;
                failed += 1;
            }
        }
    }

    println!(
        "[boot] Restore complete: {restored} active, {failed} failed, {} skipped",
        total as u32 - restored - failed
    );

    // Persist updated state (disabled tunnels that failed to restore).
    if failed > 0 {
        let tunnels_slice: &[Tunnel] = &tunnels;
        if let Err(e) = save_tunnels(tunnels_slice).await {
            eprintln!("[boot] Failed to persist updated state after restore: {e}");
        }
    }
}
