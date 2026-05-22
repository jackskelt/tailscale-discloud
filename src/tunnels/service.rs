use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::net::utils::is_port_available;
use crate::storage::tunnels_json::save_tunnels;
use crate::tunnels::{SharedState, Tunnel};

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

    tracing::debug!(
        local_port = local_port,
        target = %format!("{target_host}:{target_port}"),
        listen = %listen_arg,
        connect = %connect_arg,
        "Prepared command arguments for socat"
    );

    use std::os::unix::process::CommandExt;

    let mut std_cmd = std::process::Command::new("socat");
    std_cmd.process_group(0);

    tracing::trace!("Spawning socat process in new process group");
    let mut child = Command::from(std_cmd)
        .arg(&listen_arg)
        .arg(&connect_arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(false)
        .spawn()
        .map_err(|e| {
            let msg = format!("Failed to spawn socat: {e}");
            tracing::error!(error = %e, "Spawn error");
            msg
        })?;

    let pid = child.id().ok_or_else(|| {
        let msg = "Failed to obtain socat PID".to_string();
        tracing::error!(msg);
        msg
    })?;

    tracing::debug!(
        pid = pid,
        local_port = local_port,
        "Spawned socat process, waiting to verify it stays alive"
    );

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
            let msg = format!("PID {pid} exited immediately — {detail}");
            tracing::error!(pid = pid, exit_status = %status, stderr = %stderr_output, "socat exited immediately");
            Err(msg)
        }
        Ok(None) => {
            tracing::debug!(pid = pid, "socat process verified alive and active");

            tokio::spawn(async move {
                tracing::trace!(pid = pid, "Monitoring socat process wait status");
                match child.wait().await {
                    Ok(status) => {
                        if status.success() {
                            tracing::debug!(pid = pid, exit_status = %status, "socat process exited cleanly");
                        } else {
                            tracing::error!(pid = pid, exit_status = %status, "socat process exited with error");
                        }
                    }
                    Err(e) => {
                        tracing::error!(pid = pid, error = %e, "Error waiting for socat process")
                    }
                }
            });

            Ok(pid)
        }
        Err(e) => {
            tracing::error!(pid = pid, error = %e, "Failed to try_wait socat process; killing it");
            let _ = Command::new("kill")
                .arg("-9")
                .arg(format!("-{}", pid))
                .output()
                .await;
            let msg = format!("Failed to check PID {pid} status: {e}");
            Err(msg)
        }
    }
}

/// Kill a socat process AND all its children by targeting the Process Group (PGID).
pub async fn kill_socat(pid: u32) -> Result<(), String> {
    tracing::debug!(pid = pid, "Attempting to kill process group");

    let status = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!("kill -9 -{}", pid))
        .status()
        .await
        .map_err(|e| {
            let msg = format!("Failed to execute shell kill for PGID {pid}: {e}");
            tracing::error!(pid = pid, error = %e, "Failed to execute kill command");
            msg
        })?;

    if status.success() {
        tracing::debug!(pid = pid, "Process group killed successfully");
    } else {
        tracing::warn!(pid = pid, exit_status = %status, "kill command completed with non-zero status");
    }

    Ok(())
}

/// Restore tunnels on boot: for each enabled tunnel whose port is free,
/// attempt **once** to spawn socat.  If the spawn fails the tunnel is
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
            "Spawning socat process for tunnel"
        );
        match spawn_socat(tunnel.local_port, &tunnel.target_host, tunnel.target_port).await {
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
