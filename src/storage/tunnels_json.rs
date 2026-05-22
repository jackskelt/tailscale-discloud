use crate::tunnels::Tunnel;

fn tunnels_path() -> String {
    std::env::var("TUNNELS_PATH").unwrap_or_else(|_| "./tunnels.json".to_string())
}

/// Load tunnels from the JSON persistence file.
/// Returns an empty vec if the file doesn't exist or is invalid.
pub async fn load_tunnels() -> Vec<Tunnel> {
    let path = tunnels_path();
    tracing::debug!(path = %path, "Loading persisted tunnels from disk");

    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => {
            let tunnels: Vec<Tunnel> = serde_json::from_str(&contents).unwrap_or_default();
            tracing::debug!(
                count = tunnels.len(),
                "Successfully loaded tunnels from disk"
            );
            tunnels
        }
        Err(e) => {
            tracing::warn!(path = %path, error = %e, "Could not read tunnels file; starting with empty list");
            Vec::new()
        }
    }
}

/// Persist the current tunnel list to disk.
/// PID fields are skipped during serialization automatically.
pub async fn save_tunnels(tunnels: &[Tunnel]) -> Result<(), String> {
    let path = tunnels_path();
    tracing::debug!(path = %path, count = tunnels.len(), "Saving tunnels to disk");

    let json = serde_json::to_string_pretty(tunnels).map_err(|e| {
        let msg = format!("JSON serialization error: {e}");
        tracing::error!(error = %e, "Failed to serialize tunnels to JSON");
        msg
    })?;

    tokio::fs::write(&path, json).await.map_err(|e| {
        let msg = format!("Failed to write {path}: {e}");
        tracing::error!(path = %path, error = %e, "Failed to write tunnels JSON to file");
        msg
    })?;

    tracing::debug!(path = %path, "Successfully saved tunnels to disk");
    Ok(())
}
