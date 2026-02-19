//! Proxy server — hyper-based transparent proxy hot path.
//!
//! Architecture (from rust-proxy skill):
//! - Uses hyper directly, NOT axum, for the hot path
//! - SSE responses stream through without buffering
//! - Request body tee: read once, clone bytes for analysis
//! - All analysis is fire-and-forget from the forwarding path
//! - Fail-open: internal errors never block client traffic

pub mod agent_detect;
pub mod handler;
pub mod interceptor;
pub mod streaming;
pub mod upstream;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::events::{EventSender, Metrics};
use interceptor::InterceptorState;

/// Start the hyper proxy server.
///
/// Binds to `addr` and serves all incoming connections through `handler::proxy_handler`.
/// The `event_sender` is shared across all connections via Arc.
/// The `metrics` Arc is shared with the management API for real counter reads.
pub async fn serve(
    addr: SocketAddr,
    event_sender: EventSender,
    metrics: Arc<Metrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("proxy listening on {}", addr);

    // Shared interceptor state — one instance for the whole server
    let state = Arc::new(InterceptorState::new(event_sender, metrics));

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);

        // Clone Arc for this connection
        let state_clone = Arc::clone(&state);

        tokio::spawn(async move {
            let svc = hyper::service::service_fn(move |req| {
                let state = Arc::clone(&state_clone);
                handler::proxy_handler(req, peer_addr, state)
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                // Log but do not propagate — fail-open design
                tracing::debug!("connection error from {}: {}", peer_addr, e);
            }
        });
    }
}
