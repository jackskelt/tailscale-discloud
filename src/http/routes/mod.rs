mod config;
mod test;
mod tunnels;

use axum::{routing::get, routing::post, routing::put, Router};
use tower_http::services::ServeDir;

use crate::tunnels::SharedState;

pub fn router(state: SharedState) -> Router {
    // Serve static frontend files from public/
    let serve_dir = ServeDir::new("./public/");

    Router::new()
        .route("/api/config", get(config::get_config))
        .route(
            "/api/tunnels",
            get(tunnels::list_tunnels).post(tunnels::create_tunnel),
        )
        .route(
            "/api/tunnels/:id",
            put(tunnels::update_tunnel).delete(tunnels::delete_tunnel),
        )
        .route("/api/test", post(test::test_endpoint))
        .with_state(state)
        .fallback_service(serve_dir)
}
