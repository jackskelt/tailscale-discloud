use hyper_util::client::legacy::Client;
use hyperlocal::UnixConnector;
use std::sync::OnceLock;

pub const TAILSCALED_SOCKET_ENV: &str = "TAILSCALE_SOCKET";
pub const DEFAULT_TAILSCALED_SOCKET: &str = "/var/run/tailscale/tailscaled.sock";
pub const LOCALAPI_HOST_HEADER: &str = "local-tailscaled.sock";

static CLIENT: OnceLock<Client<UnixConnector, http_body_util::Full<bytes::Bytes>>> =
    OnceLock::new();

pub fn tailscaled_socket_path() -> String {
    std::env::var(TAILSCALED_SOCKET_ENV).unwrap_or_else(|_| DEFAULT_TAILSCALED_SOCKET.to_string())
}

pub fn get_client() -> &'static Client<UnixConnector, http_body_util::Full<bytes::Bytes>> {
    CLIENT
        .get_or_init(|| Client::builder(hyper_util::rt::TokioExecutor::new()).build(UnixConnector))
}

pub async fn request(
    method: hyper::Method,
    path: &str,
    body_bytes: bytes::Bytes,
) -> Result<bytes::Bytes, String> {
    use http_body_util::{BodyExt, Full};
    use hyper::Request;
    use hyperlocal::Uri;

    let socket_path = tailscaled_socket_path();
    let client = get_client();
    let uri: hyper::Uri = Uri::new(socket_path, path).into();

    let mut req_builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Host", LOCALAPI_HOST_HEADER);

    if !body_bytes.is_empty() {
        req_builder = req_builder.header("Content-Type", "application/json");
    }

    let req = req_builder.body(Full::new(body_bytes)).map_err(|e| {
        let msg = format!("[localapi] Request build failed for {path}: {e}");
        tracing::debug!(error = %e, "LocalAPI request build failed");
        msg
    })?;

    tracing::debug!(
        method = ?req.method(),
        path = path,
        "Sending request to local tailscaled socket"
    );
    let res = client.request(req).await.map_err(|e| {
        let msg = format!("[localapi] Request failed for {path}: {e}");
        tracing::debug!(error = %e, "LocalAPI connection or request failed");
        msg
    })?;

    let status = res.status();
    let res_body = res
        .into_body()
        .collect()
        .await
        .map_err(|e| {
            let msg = format!("[localapi] Read body failed for {path}: {e}");
            tracing::debug!(error = %e, "Failed to read LocalAPI response body");
            msg
        })?
        .to_bytes();

    if !status.is_success() && status != hyper::StatusCode::NO_CONTENT {
        let body_str = String::from_utf8_lossy(&res_body);
        let msg = format!("[localapi] HTTP {status} returned for {path}: {body_str}");
        tracing::debug!(http_status = %status, body = %body_str, "LocalAPI returned error status");
        return Err(msg);
    }

    Ok(res_body)
}
