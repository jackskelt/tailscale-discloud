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
use crate::state::{load_tunnels, restore_tunnels, SharedState};

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

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
}
