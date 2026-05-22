use std::sync::Arc;

use axum::Router;
use tokio::sync::RwLock;

use tailscale_tunnel_manager::http::routes::router;
use tailscale_tunnel_manager::storage::tunnels_json::load_tunnels;
use tailscale_tunnel_manager::tunnels::service::restore_tunnels;
use tailscale_tunnel_manager::tunnels::SharedState;

use tailscale_tunnel_manager::tailscale::localapi::get_local_node_info;

#[tokio::main]
async fn main() {
    let _guard = tailscale_tunnel_manager::logging::init_logging();

    tracing::info!(target: "tailscale_tunnel_manager", "Tailscale Tunnel Manager starting...");

    // Load persisted tunnels from disk
    let tunnels = load_tunnels().await;
    tracing::info!(target: "tailscale_tunnel_manager", "Loaded {} tunnel(s) from disk", tunnels.len());

    let state: SharedState = Arc::new(RwLock::new(tunnels));

    // Restore enabled tunnels
    restore_tunnels(&state).await;

    // Build full application with flat routes
    let app: Router = router(state);

    let bind_addr = "0.0.0.0:3000";
    tracing::info!("Listening on {}", bind_addr);

    match get_local_node_info().await {
        Ok(config) => {
            tailscale_tunnel_manager::logging::print_node_box(&config);
        }
        Err(e) => {
            tracing::error!(target: "tailscale_tunnel_manager", "Failed to read LocalAPI node info: {}", e)
        }
    }

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
}
