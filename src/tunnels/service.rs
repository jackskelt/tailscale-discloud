use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::net::utils::is_port_available;
use crate::storage::tunnels_json::save_tunnels;
use crate::tunnels::{SharedState, Tunnel};

#[derive(Debug)]
pub enum TunnelSpawnError {
    AddrInUse(u16),
    Other(String),
}

impl std::fmt::Display for TunnelSpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddrInUse(port) => write!(f, "Port {port} is already in use on the system"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

static ACTIVE_TUNNELS: OnceLock<tokio::sync::Mutex<HashMap<u16, oneshot::Sender<()>>>> =
    OnceLock::new();

fn get_active_tunnels() -> &'static tokio::sync::Mutex<HashMap<u16, oneshot::Sender<()>>> {
    ACTIVE_TUNNELS.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

/// Spawn a TCP tunnel process that forwards `local_port` -> `target_host:target_port`.
pub async fn spawn_tcp_tunnel(
    local_port: u16,
    target_host: &str,
    target_port: u16,
) -> Result<u32, TunnelSpawnError> {
    let target_host_str = target_host.to_string();
    tracing::debug!(
        local_port = local_port,
        target = %format!("{target_host_str}:{target_port}"),
        "Spawning tunnel listener"
    );

    // Bind TCP listener synchronously before spawning.
    // If the port is already in use or permission is denied, it fails immediately.
    let listener = TcpListener::bind(("0.0.0.0", local_port))
        .await
        .map_err(|e| {
            tracing::debug!(local_port = local_port, error = %e, "Bind error");
            if e.kind() == std::io::ErrorKind::AddrInUse {
                TunnelSpawnError::AddrInUse(local_port)
            } else {
                TunnelSpawnError::Other(format!("Failed to bind local port {local_port}: {e}"))
            }
        })?;

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let pid = local_port as u32;
    {
        let mut active = get_active_tunnels().lock().await;
        if active.insert(local_port, shutdown_tx).is_some() {
            tracing::warn!(
                local_port = local_port,
                "Replaced existing tunnel registration on this port"
            );
        }
    }

    tracing::debug!(
        pid = pid,
        local_port = local_port,
        "tunnel spawned successfully"
    );

    tokio::spawn(async move {
        tracing::trace!(local_port = local_port, "tunnel background loop started");

        let (conn_shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    tracing::trace!(local_port = local_port, "tunnel received shutdown signal");
                    break;
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((mut client_stream, client_addr)) => {
                            tracing::trace!(
                                local_port = local_port,
                                client_addr = %client_addr,
                                "Accepted client connection"
                            );

                            let target_host_clone = target_host_str.clone();
                            let mut conn_shutdown_rx = conn_shutdown_tx.subscribe();

                            tokio::spawn(async move {
                                let target_addr = format!("{target_host_clone}:{target_port}");
                                let mut target_stream = match tokio::net::TcpStream::connect(&target_addr).await {
                                    Ok(s) => s,
                                    Err(e) => {
                                        tracing::trace!(
                                            client_addr = %client_addr,
                                            target_addr = %target_addr,
                                            error = %e,
                                            "Failed to connect to target host"
                                        );
                                        return;
                                    }
                                };

                                tracing::trace!(
                                    client_addr = %client_addr,
                                    target_addr = %target_addr,
                                    "Connected to target, starting bidirectional copy"
                                );

                                tokio::select! {
                                    res = tokio::io::copy_bidirectional(&mut client_stream, &mut target_stream) => {
                                        match res {
                                            Ok((c2t, t2c)) => {
                                                tracing::trace!(
                                                    client_addr = %client_addr,
                                                    c2t = c2t,
                                                    t2c = t2c,
                                                    "Connection closed normally"
                                                );
                                            }
                                            Err(e) => {
                                                tracing::trace!(
                                                    client_addr = %client_addr,
                                                    error = %e,
                                                    "Connection error during copy"
                                                );
                                            }
                                        }
                                    }
                                    _ = conn_shutdown_rx.recv() => {
                                        tracing::trace!(
                                            client_addr = %client_addr,
                                            "Active connection shut down due to tunnel shutdown"
                                        );
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            tracing::trace!(local_port = local_port, error = %e, "Failed to accept connection");
                        }
                    }
                }
            }
        }

        let _ = conn_shutdown_tx.send(());
        tracing::trace!(local_port = local_port, "tunnel background loop stopped");
    });

    Ok(pid)
}

/// Kill a TCP tunnel listener by local port (derived from mock pid).
pub async fn kill_tunnel(pid: u32) -> Result<(), String> {
    tracing::debug!(pid = pid, "Attempting to stop tunnel");

    let local_port = pid as u16;
    let mut active = get_active_tunnels().lock().await;
    if let Some(shutdown_tx) = active.remove(&local_port) {
        let _ = shutdown_tx.send(());
        tracing::debug!(pid = pid, "Shutdown signal sent to tunnel");
    } else {
        tracing::warn!(pid = pid, "No active tunnel found for this port");
    }

    Ok(())
}

/// Restore tunnels on boot: for each enabled tunnel whose port is free,
/// attempt **once** to spawn forwarder. If the spawn fails the tunnel is
/// marked `enabled = false` so we never retry in an infinite loop.
pub async fn restore_tunnels(state: &SharedState) {
    let mut tunnels = state.write().await;
    let total = tunnels.len();
    let mut restored = 0u32;
    let mut failed = 0u32;

    tracing::info!(total = total, "Restoring active tunnels on startup");

    for tunnel in tunnels.iter_mut() {
        if !tunnel.enabled {
            tracing::debug!(
                name = %tunnel.name,
                local_port = tunnel.local_port,
                "Tunnel is disabled; skipping"
            );
            tunnel.pid = None;
            continue;
        }

        tracing::trace!(
            name = %tunnel.name,
            local_port = tunnel.local_port,
            "Checking system port availability"
        );
        if !is_port_available(tunnel.local_port).await {
            tracing::warn!(
                name = %tunnel.name,
                local_port = tunnel.local_port,
                "Port is already in use; disabling tunnel"
            );
            tunnel.enabled = false;
            tunnel.pid = None;
            failed += 1;
            continue;
        }

        tracing::debug!(
            name = %tunnel.name,
            local_port = tunnel.local_port,
            target = %format!("{}:{}", tunnel.target_host, tunnel.target_port),
            "Spawning tunnel process for tunnel"
        );
        match spawn_tcp_tunnel(tunnel.local_port, &tunnel.target_host, tunnel.target_port).await {
            Ok(pid) => {
                tracing::debug!(
                    name = %tunnel.name,
                    pid = pid,
                    "Tunnel successfully restored"
                );
                tunnel.pid = Some(pid);
                restored += 1;
            }
            Err(e) => {
                tracing::warn!(
                    name = %tunnel.name,
                    error = %e,
                    "Failed to restore tunnel; marking as disabled"
                );
                tunnel.enabled = false;
                tunnel.pid = None;
                failed += 1;
            }
        }
    }

    tracing::info!(
        restored = restored,
        failed = failed,
        skipped = (total as u32 - restored - failed),
        "Boot-time tunnel restoration complete"
    );

    // Persist updated state (disabled tunnels that failed to restore).
    if failed > 0 {
        let tunnels_slice: &[Tunnel] = &tunnels;
        tracing::debug!("Persisting updated tunnel configurations after failures");
        if let Err(e) = save_tunnels(tunnels_slice).await {
            tracing::error!(error = %e, "Failed to persist updated state after restore");
        }
    }
}
