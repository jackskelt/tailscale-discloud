pub mod client;
pub mod endpoints;
pub mod models;

pub use client::tailscaled_socket_path;
pub use endpoints::{get_local_node_info, get_local_status, get_prefs, start};
pub use models::{LocalNodeInfo, LocalStatus, Options, Prefs};
