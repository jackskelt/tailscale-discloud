use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::tailscale::localapi::{get_local_status, get_prefs, start, Options, Prefs};

/// Spawn the `tailscaled` daemon as a child process
pub async fn start_tailscaled() -> Result<tokio::process::Child, String> {
    let tailscale_state = std::env::var("TAILSCALE_STATE")
        .unwrap_or_else(|_| "/home/discloud/tailscale.state".to_string());

    tracing::debug!("Starting tailscaled child process");

    let mut cmd = Command::new("tailscaled");
    cmd.arg("--tun=userspace-networking")
        .arg(format!("--state={}", tailscale_state))
        .arg("--socket=/var/run/tailscale/tailscaled.sock")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn tailscaled: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or("Failed to open tailscaled stdout")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("Failed to open tailscaled stderr")?;

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            tracing::info!(target: "tailscaled", "{}", line);
        }
    });

    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            tracing::info!(target: "tailscaled", "{}", line);
        }
    });

    Ok(child)
}

/// Tailscale initialization flow:
/// 1. Spawn tailscaled
/// 2. Wait for the LocalAPI socket to become available
/// 3. Trigger interactive login via LocalAPI
/// 4. Poll status until authentication completes
pub async fn init_tailscale_flow() -> Result<tokio::process::Child, String> {
    let tailscaled_child = start_tailscaled().await?;

    // Wait for the LocalAPI socket to become responsive
    let mut waiting_socket = false;
    let initial_status = loop {
        match get_local_status().await {
            Ok(status) => break status,
            Err(_) => {
                if !waiting_socket {
                    tracing::debug!("Waiting for tailscaled socket to be ready");
                    waiting_socket = true;
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    };

    let mut last_auth_url: Option<String> = initial_status.auth_url.filter(|u| !u.is_empty());

    // Query current preferences to modify and send to /localapi/v0/start
    let mut prefs = match get_prefs().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                "Failed to query current prefs from tailscaled: {}; using default prefs",
                e
            );
            Prefs::default()
        }
    };

    let hostname =
        std::env::var("TAILSCALE_HOSTNAME").unwrap_or_else(|_| "tailscale-discloud".to_string());
    let auth_key = std::env::var("TAILSCALE_AUTHKEY")
        .ok()
        .filter(|s| !s.is_empty());

    prefs.hostname = Some(hostname);
    prefs.route_all = Some(true);
    prefs.corp_dns = Some(true);
    prefs.want_running = Some(true);

    let opts = Options {
        frontend_log_id: None,
        update_prefs: Some(prefs),
        auth_key,
    };

    tracing::debug!("Starting and configuring Tailscale via LocalAPI");
    start(opts).await?;

    let mut last_state: Option<String> = None;

    loop {
        match get_local_status().await {
            Ok(status) => {
                let state = status.backend_state;
                let state_changed = Some(&state) != last_state.as_ref();
                if state_changed {
                    tracing::trace!("Tailscale state transition: {:?}", state);
                    last_state = Some(state.clone());
                }

                if state == "Running" {
                    tracing::info!("Tailscale is authenticated and running.");
                    break;
                } else if state == "NeedsLogin" {
                    if let Some(auth_url) = status.auth_url {
                        if Some(&auth_url) != last_auth_url.as_ref() && !auth_url.is_empty() {
                            let clickable = crate::logging::clickable_terminal_link(&auth_url);
                            crate::logging::print_box(&[
                                "To authenticate, visit:".to_string(),
                                format!("  {clickable}"),
                            ]);
                            last_auth_url = Some(auth_url);
                        }
                    }
                } else if state == "NeedsMachineAuth" && state_changed {
                    tracing::warn!("Machine requires authorization from the Tailnet administrator.")
                }
            }
            Err(e) => {
                tracing::error!("Failed to query status: {}", e);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Ok(tailscaled_child)
}
