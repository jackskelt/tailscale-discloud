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
