use std::sync::Arc;

use axum::Router;
use tokio::sync::RwLock;

use tailscale_tunnel_manager::http::routes::router;
use tailscale_tunnel_manager::storage::tunnels_json::load_tunnels;
use tailscale_tunnel_manager::tunnels::service::restore_tunnels;
use tailscale_tunnel_manager::tunnels::SharedState;

use tailscale_tunnel_manager::tailscale::localapi::get_local_node_info;

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

    // Build full application with flat routes
    let app: Router = router(state);

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
