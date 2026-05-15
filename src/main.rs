mod models;
mod routes;
mod state;

use std::sync::Arc;

use axum::{routing::get, routing::post, routing::put, Router};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::routes::{
    create_tunnel, delete_tunnel, get_config, list_tunnels, test_endpoint, update_tunnel,
};
use crate::state::{get_local_node_info, load_tunnels, restore_tunnels, SharedState};

fn clickable_terminal_link(url: &str) -> String {
    // For terminals that support OSC 8 hyperlinks (most Unix terminals), format the URL as a clickable link.
    format!("\x1b]8;;{url}\x1b\\{url}\x1b]8;;\x1b\\")
}

fn print_link(label: &str, url: &str) {
    println!("  - {label}: {}", clickable_terminal_link(url));
}

#[tokio::main]
async fn main() {
    println!("[main] Tailscale Tunnel Manager starting...");

    // Load persisted tunnels from disk
    let tunnels = load_tunnels().await;
    println!("[main] Loaded {} tunnel(s) from disk", tunnels.len());

    let state: SharedState = Arc::new(RwLock::new(tunnels));

    // Restore enabled tunnels
    restore_tunnels(&state).await;

    // Serve static frontend files from public/
    let serve_dir = ServeDir::new("./public/");

    // Build full application with flat routes
    let app = Router::new()
        .route("/api/config", get(get_config))
        .route("/api/tunnels", get(list_tunnels).post(create_tunnel))
        .route("/api/tunnels/:id", put(update_tunnel).delete(delete_tunnel))
        .route("/api/test", post(test_endpoint))
        .with_state(state)
        .fallback_service(serve_dir);

    let bind_addr = "0.0.0.0:3000";
    println!("[main] Listening on {bind_addr}");

    match get_local_node_info().await {
        Ok(config) => {
            println!("[main] You can connect to this node using:");

            if !config.dns_name.is_empty() {
                print_link("DNSName", &format!("http://{}:3000/", config.dns_name));
            }
            if !config.magicdns_hostname.is_empty() {
                print_link(
                    "MagicDNS",
                    &format!("http://{}:3000/", config.magicdns_hostname),
                );
            }
            if !config.ipv4.is_empty() {
                print_link("IPv4", &format!("http://{}:3000/", config.ipv4));
            }
            if !config.ipv6.is_empty() {
                print_link("IPv6", &format!("http://[{}]:3000/", config.ipv6));
            }
        }
        Err(e) => eprintln!("[main] Failed to read LocalAPI node info: {e}"),
    }

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
}
