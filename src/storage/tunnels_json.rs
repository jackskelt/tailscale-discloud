use crate::tunnels::Tunnel;

fn tunnels_path() -> String {
    std::env::var("TUNNELS_PATH").unwrap_or_else(|_| "./tunnels.json".to_string())
}

/// Load tunnels from the JSON persistence file.
/// Returns an empty vec if the file doesn't exist or is invalid.
pub async fn load_tunnels() -> Vec<Tunnel> {
    let path = tunnels_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => {
            let tunnels: Vec<Tunnel> = serde_json::from_str(&contents).unwrap_or_default();
            println!("[state] Loaded {} tunnel(s) from {}", tunnels.len(), path);
            tunnels
        }
        Err(e) => {
            eprintln!("[state] Could not read {path}: {e} — starting with empty list");
            Vec::new()
        }
    }
}

/// Persist the current tunnel list to disk.
/// PID fields are skipped during serialization automatically.
pub async fn save_tunnels(tunnels: &[Tunnel]) -> Result<(), String> {
    let path = tunnels_path();
    let json = serde_json::to_string_pretty(tunnels).map_err(|e| {
        let msg = format!("[state] JSON serialization error: {e}");
        eprintln!("{msg}");
        msg
    })?;

    tokio::fs::write(&path, json).await.map_err(|e| {
        let msg = format!("[state] Failed to write {path}: {e}");
        eprintln!("{msg}");
        msg
    })?;

    println!("[state] Persisted {} tunnel(s) to {}", tunnels.len(), path);
    Ok(())
}
