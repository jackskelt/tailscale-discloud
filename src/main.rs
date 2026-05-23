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
    let _log_guards = tailscale_tunnel_manager::logging::init_logging();

    let args: Vec<String> = std::env::args().collect();
    let is_prod = args.contains(&"--prod".to_string())
        || std::env::var("PRODUCTION").unwrap_or_default() == "true";

    let _tailscaled_child = if is_prod {
        tracing::info!("Initializing Tailscale processes...");
        match tailscale_tunnel_manager::tailscale::process::init_tailscale_flow().await {
            Ok(child) => Some(child),
            Err(e) => {
                tracing::error!("Failed to initialize Tailscale: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };
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
            if is_prod {
                tracing::error!(target: "tailscale_tunnel_manager", "Failed to read LocalAPI node info: {}", e);
            } else {
                tracing::error!(target: "tailscale_tunnel_manager", "Local Tailscale socket not accessible in development mode: {}", e);
            }
        }
    }

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
}
