use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;

use crate::models::{
    ApiErrorResponse, ApiMessage, ConfigResponse, CreateTunnelRequest, ReachabilityResult,
    TestConnectionRequest, TestConnectionResponse, TunnelListItem, TunnelResponse,
    UpdateTunnelRequest,
};
use crate::state::{
    check_target_reachability, connection_url_for, get_hostname, is_loopback_host,
    is_port_available, kill_socat, save_tunnels, spawn_socat, test_connection, SharedState,
};

// ─── Helpers ─────────────────────────────────────────────────────────────

/// Build an `ApiErrorResponse` from an i18n key with no parameters.
fn api_err(id: &str) -> ApiErrorResponse {
    ApiErrorResponse {
        error: ApiMessage::new(id),
    }
}

/// Build an `ApiErrorResponse` from an i18n key with parameters.
fn api_err_params(id: &str, params: HashMap<String, serde_json::Value>) -> ApiErrorResponse {
    ApiErrorResponse {
        error: ApiMessage::with_params(id, params),
    }
}

/// Shortcut: create a single-entry params map.
fn params1(key: &str, val: impl Into<serde_json::Value>) -> HashMap<String, serde_json::Value> {
    let mut m = HashMap::new();
    m.insert(key.to_string(), val.into());
    m
}

/// Shortcut: create a two-entry params map.
fn params2(
    k1: &str,
    v1: impl Into<serde_json::Value>,
    k2: &str,
    v2: impl Into<serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut m = HashMap::new();
    m.insert(k1.to_string(), v1.into());
    m.insert(k2.to_string(), v2.into());
    m
}

// ─── Type aliases for route return types ─────────────────────────────────

type ApiResult<T> = Result<T, (StatusCode, Json<ApiErrorResponse>)>;

// ─── GET /api/config ─────────────────────────────────────────────────────

pub async fn get_config() -> Json<ConfigResponse> {
    let hostname = get_hostname();
    let version = env!("CARGO_PKG_VERSION").to_string();
    println!("[GET /api/config] hostname={hostname} version={version}");
    Json(ConfigResponse { hostname, version })
}

// ─── GET /api/tunnels ────────────────────────────────────────────────────

pub async fn list_tunnels(State(state): State<SharedState>) -> Json<Vec<TunnelListItem>> {
    let tunnels = state.read().await;
    println!("[GET /api/tunnels] Returning {} tunnel(s)", tunnels.len());

    let items: Vec<TunnelListItem> = tunnels
        .iter()
        .map(|t| TunnelListItem {
            connection_url: connection_url_for(t),
            tunnel: t.clone(),
        })
        .collect();

    Json(items)
}

// ─── POST /api/tunnels ──────────────────────────────────────────────────

pub async fn create_tunnel(
    State(state): State<SharedState>,
    Json(payload): Json<CreateTunnelRequest>,
) -> ApiResult<(StatusCode, Json<TunnelResponse>)> {
    println!(
        "[POST /api/tunnels] name='{}' local_port={} target={}:{} enabled={}",
        payload.name, payload.local_port, payload.target_host, payload.target_port, payload.enabled
    );

    // ── Validation ──────────────────────────────────────────────────────
    if payload.name.trim().is_empty() {
        eprintln!("[POST /api/tunnels] Rejected: empty name");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(api_err("api.error.name_empty")),
        ));
    }

    if payload.target_host.trim().is_empty() {
        eprintln!("[POST /api/tunnels] Rejected: empty target_host");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(api_err("api.error.target_host_empty")),
        ));
    }

    if payload.local_port == 0 {
        eprintln!("[POST /api/tunnels] Rejected: local_port is 0");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(api_err("api.error.local_port_range")),
        ));
    }

    if payload.target_port == 0 {
        eprintln!("[POST /api/tunnels] Rejected: target_port is 0");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(api_err("api.error.target_port_range")),
        ));
    }

    // ── Self-loop detection ─────────────────────────────────────────────
    if payload.local_port == payload.target_port && is_loopback_host(&payload.target_host) {
        eprintln!(
            "[POST /api/tunnels] Rejected: self-loop (localhost:{} -> localhost:{})",
            payload.local_port, payload.target_port
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(api_err_params(
                "api.error.self_loop",
                params1("port", payload.local_port),
            )),
        ));
    }

    // ── Port availability (system-level) ────────────────────────────────
    if !is_port_available(payload.local_port).await {
        eprintln!(
            "[POST /api/tunnels] Rejected: port {} is in use on the system",
            payload.local_port
        );
        return Err((
            StatusCode::CONFLICT,
            Json(api_err_params(
                "api.error.port_in_use",
                params1("port", payload.local_port),
            )),
        ));
    }

    let mut tunnels = state.write().await;

    // ── Port availability (within our state) ────────────────────────────
    if tunnels.iter().any(|t| t.local_port == payload.local_port) {
        eprintln!(
            "[POST /api/tunnels] Rejected: port {} already assigned to another tunnel",
            payload.local_port
        );
        return Err((
            StatusCode::CONFLICT,
            Json(api_err_params(
                "api.error.port_assigned",
                params1("port", payload.local_port),
            )),
        ));
    }

    // ── Build tunnel struct ─────────────────────────────────────────────
    let mut tunnel = crate::models::Tunnel {
        id: Uuid::new_v4().to_string(),
        name: payload.name.trim().to_string(),
        local_port: payload.local_port,
        target_host: payload.target_host.trim().to_string(),
        target_port: payload.target_port,
        enabled: payload.enabled,
        pid: None,
        warning_id: None,
    };

    let mut warning: Option<ApiMessage> = None;

    // ── Spawn socat if enabled (with reachability pre-check) ────────────
    if tunnel.enabled {
        // Pre-flight reachability check
        match check_target_reachability(&tunnel.target_host, tunnel.target_port).await {
            ReachabilityResult::Reachable => {
                // All good — clear any stale warning.
                tunnel.warning_id = None;
            }
            ReachabilityResult::HostReachablePortClosed => {
                // Host responds but nothing on that port — warn, continue.
                println!(
                    "[POST /api/tunnels] Warning: {}:{} — host reachable but port closed",
                    tunnel.target_host, tunnel.target_port
                );
                tunnel.warning_id = Some("api.warning.port_closed".to_string());
                warning = Some(ApiMessage::with_params(
                    "api.warning.port_closed",
                    params2(
                        "host",
                        tunnel.target_host.clone(),
                        "port",
                        tunnel.target_port,
                    ),
                ));
            }
            ReachabilityResult::HostUnreachable(reason) => {
                eprintln!(
                    "[POST /api/tunnels] Rejected: host {} unreachable — {reason}",
                    tunnel.target_host
                );
                return Err((
                    StatusCode::BAD_GATEWAY,
                    Json(api_err_params(
                        "api.error.host_unreachable",
                        params2("host", tunnel.target_host.clone(), "reason", reason),
                    )),
                ));
            }
        }

        match spawn_socat(tunnel.local_port, &tunnel.target_host, tunnel.target_port).await {
            Ok(pid) => {
                println!(
                    "[POST /api/tunnels] socat started for '{}' with PID {}",
                    tunnel.name, pid
                );
                tunnel.pid = Some(pid);
            }
            Err(e) => {
                eprintln!(
                    "[POST /api/tunnels] socat failed for '{}': {e}",
                    tunnel.name
                );
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(api_err_params(
                        "api.error.socat_failed",
                        params1("detail", e),
                    )),
                ));
            }
        }
    }

    tunnels.push(tunnel.clone());

    // ── Persist ─────────────────────────────────────────────────────────
    if let Err(e) = save_tunnels(&tunnels).await {
        eprintln!("[POST /api/tunnels] Persistence failed (tunnel IS running): {e}");
    }

    println!(
        "[POST /api/tunnels] Created tunnel '{}' id={}",
        tunnel.name, tunnel.id
    );

    let response = TunnelResponse {
        connection_url: connection_url_for(&tunnel),
        tunnel,
        warning,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// ─── PUT /api/tunnels/:id ───────────────────────────────────────────────

pub async fn update_tunnel(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTunnelRequest>,
) -> ApiResult<Json<TunnelResponse>> {
    println!("[PUT /api/tunnels/{id}] payload: {payload:?}");

    let mut tunnels = state.write().await;

    let index = tunnels.iter().position(|t| t.id == id).ok_or_else(|| {
        eprintln!("[PUT /api/tunnels/{id}] Not found");
        (
            StatusCode::NOT_FOUND,
            Json(api_err_params(
                "api.error.tunnel_not_found",
                params1("id", id.clone()),
            )),
        )
    })?;

    let tunnel = &tunnels[index];

    // ── Compute new values ──────────────────────────────────────────────
    let new_name = payload
        .name
        .as_deref()
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| tunnel.name.clone());
    let new_local_port = payload.local_port.unwrap_or(tunnel.local_port);
    let new_target_host = payload
        .target_host
        .as_deref()
        .map(|h| h.trim().to_string())
        .unwrap_or_else(|| tunnel.target_host.clone());
    let new_target_port = payload.target_port.unwrap_or(tunnel.target_port);
    let new_enabled = payload.enabled.unwrap_or(tunnel.enabled);

    // ── Port validation if changed ──────────────────────────────────────
    if new_local_port != tunnel.local_port {
        if tunnels
            .iter()
            .enumerate()
            .any(|(i, t)| i != index && t.local_port == new_local_port)
        {
            eprintln!(
                "[PUT /api/tunnels/{id}] Port {} already assigned to another tunnel",
                new_local_port
            );
            return Err((
                StatusCode::CONFLICT,
                Json(api_err_params(
                    "api.error.port_assigned",
                    params1("port", new_local_port),
                )),
            ));
        }

        if !is_port_available(new_local_port).await {
            eprintln!(
                "[PUT /api/tunnels/{id}] Port {} in use on the system",
                new_local_port
            );
            return Err((
                StatusCode::CONFLICT,
                Json(api_err_params(
                    "api.error.port_in_use",
                    params1("port", new_local_port),
                )),
            ));
        }
    }

    // ── Self-loop detection ─────────────────────────────────────────────
    if new_local_port == new_target_port && is_loopback_host(&new_target_host) {
        eprintln!(
            "[PUT /api/tunnels/{id}] Rejected: self-loop (localhost:{} -> localhost:{})",
            new_local_port, new_target_port
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(api_err_params(
                "api.error.self_loop",
                params1("port", new_local_port),
            )),
        ));
    }

    // Determine whether the tunnel is transitioning to enabled or the
    // target changed while enabled — in either case we need a
    // reachability check.
    let was_enabled = tunnel.enabled;
    let target_changed =
        new_target_host != tunnel.target_host || new_target_port != tunnel.target_port;
    let needs_reachability_check =
        new_enabled && (!was_enabled || target_changed || new_local_port != tunnel.local_port);

    let mut warning: Option<ApiMessage> = None;

    // Track the warning key to persist on the tunnel struct.
    let mut warning_key: Option<String> = None;

    if needs_reachability_check {
        match check_target_reachability(&new_target_host, new_target_port).await {
            ReachabilityResult::Reachable => {
                // Clear any previous warning.
                warning_key = None;
            }
            ReachabilityResult::HostReachablePortClosed => {
                println!(
                    "[PUT /api/tunnels/{id}] Warning: {new_target_host}:{new_target_port} — host reachable but port closed"
                );
                warning_key = Some("api.warning.port_closed".to_string());
                warning = Some(ApiMessage::with_params(
                    "api.warning.port_closed",
                    params2("host", new_target_host.clone(), "port", new_target_port),
                ));
            }
            ReachabilityResult::HostUnreachable(reason) => {
                eprintln!(
                    "[PUT /api/tunnels/{id}] Rejected: host {new_target_host} unreachable — {reason}"
                );
                return Err((
                    StatusCode::BAD_GATEWAY,
                    Json(api_err_params(
                        "api.error.host_unreachable",
                        params2("host", new_target_host.clone(), "reason", reason),
                    )),
                ));
            }
        }
    }

    // ── Kill old socat ──────────────────────────────────────────────────
    let old_pid = tunnel.pid;
    if let Some(pid) = old_pid {
        println!("[PUT /api/tunnels/{id}] Killing old socat PID {pid}");
        if let Err(e) = kill_socat(pid).await {
            eprintln!("[PUT /api/tunnels/{id}] Failed to kill old socat PID {pid}: {e}");
        }

        // If the port didn't change, give the OS time to release it.
        if new_local_port == tunnel.local_port {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    // ── Spawn new socat if enabled ──────────────────────────────────────
    let new_pid = if new_enabled {
        match spawn_socat(new_local_port, &new_target_host, new_target_port).await {
            Ok(pid) => {
                println!("[PUT /api/tunnels/{id}] socat started PID {pid}");
                Some(pid)
            }
            Err(e) => {
                eprintln!("[PUT /api/tunnels/{id}] socat failed: {e}");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(api_err_params(
                        "api.error.socat_failed",
                        params1("detail", e),
                    )),
                ));
            }
        }
    } else {
        None
    };

    // ── Apply changes ───────────────────────────────────────────────────
    let tunnel = &mut tunnels[index];
    tunnel.name = new_name;
    tunnel.local_port = new_local_port;
    tunnel.target_host = new_target_host;
    tunnel.target_port = new_target_port;
    tunnel.enabled = new_enabled;
    tunnel.pid = new_pid;
    // Persist the warning on the tunnel (or clear it).
    if needs_reachability_check {
        tunnel.warning_id = warning_key;
    } else if !new_enabled {
        // Disabled tunnels should not carry stale warnings.
        tunnel.warning_id = None;
    }

    let updated = tunnel.clone();

    // ── Persist ─────────────────────────────────────────────────────────
    if let Err(e) = save_tunnels(&tunnels).await {
        eprintln!("[PUT /api/tunnels/{id}] Persistence failed (tunnel IS running): {e}");
    }

    println!(
        "[PUT /api/tunnels/{id}] Updated tunnel '{}' enabled={}",
        updated.name, updated.enabled
    );

    let response = TunnelResponse {
        connection_url: connection_url_for(&updated),
        tunnel: updated,
        warning,
    };

    Ok(Json(response))
}

// ─── DELETE /api/tunnels/:id ────────────────────────────────────────────

pub async fn delete_tunnel(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    println!("[DELETE /api/tunnels/{id}]");

    let mut tunnels = state.write().await;

    let index = tunnels.iter().position(|t| t.id == id).ok_or_else(|| {
        eprintln!("[DELETE /api/tunnels/{id}] Not found");
        (
            StatusCode::NOT_FOUND,
            Json(api_err_params(
                "api.error.tunnel_not_found",
                params1("id", id.clone()),
            )),
        )
    })?;

    let tunnel = &tunnels[index];
    let name = tunnel.name.clone();

    // ── Kill socat ──────────────────────────────────────────────────────
    if let Some(pid) = tunnel.pid {
        println!("[DELETE /api/tunnels/{id}] Killing socat PID {pid}");
        if let Err(e) = kill_socat(pid).await {
            eprintln!("[DELETE /api/tunnels/{id}] Failed to kill socat PID {pid}: {e}");
        }
    }

    tunnels.remove(index);

    // ── Persist ─────────────────────────────────────────────────────────
    if let Err(e) = save_tunnels(&tunnels).await {
        eprintln!("[DELETE /api/tunnels/{id}] Persistence failed: {e}");
    }

    println!("[DELETE /api/tunnels/{id}] Deleted tunnel '{name}'");

    Ok(StatusCode::NO_CONTENT)
}

// ─── POST /api/test ─────────────────────────────────────────────────────

pub async fn test_endpoint(
    Json(payload): Json<TestConnectionRequest>,
) -> Json<TestConnectionResponse> {
    println!(
        "[POST /api/test] target={}:{}",
        payload.target_host, payload.target_port
    );

    let (success, log) = test_connection(&payload.target_host, payload.target_port).await;

    println!(
        "[POST /api/test] {}:{} -> {}",
        payload.target_host,
        payload.target_port,
        if success { "OK" } else { "FAIL" }
    );

    Json(TestConnectionResponse { success, log })
}
