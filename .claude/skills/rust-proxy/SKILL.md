---
name: rust-proxy
description: Rust proxy architecture decisions for AgentMesh SaaS hot path. Use when implementing or modifying the Rust proxy layer, SSE streaming, request interception, or protocol detection.
compatibility: rust, proxy, networking, sse, hyper
---

# AgentMesh Rust Proxy Architecture

## Core Architecture Decisions

### Use hyper, NOT axum for the hot path

The proxy hot path (request interception, forwarding, response streaming) MUST use `hyper` directly. Axum adds routing overhead and abstraction layers that cost latency in a transparent proxy.

- **hyper** for: all request/response handling, body streaming, connection management
- **axum** is acceptable ONLY for: admin/management API endpoints (health checks, config reload, metrics)
- **Never** use axum's `Router` or middleware chain for proxied traffic

```rust
// CORRECT: hyper service for proxy hot path
use hyper::{Request, Response, Body};
use hyper::service::{make_service_fn, service_fn};

async fn proxy_handler(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let protocol = detect_protocol(&req);
    // ... forward and tee
}

// WRONG: Do NOT use axum Router for proxied traffic
// let app = Router::new().route("/proxy/*path", post(proxy_handler));
```

### SSE Stream-Through Pattern

For LLM streaming responses (OpenAI `stream: true`, Anthropic `stream: true`), the proxy MUST stream through without buffering the entire response.

```rust
/// Stream SSE events through the proxy, teeing each chunk for analysis.
/// The client receives bytes as soon as the upstream sends them.
/// Analysis (token counting, PII scan) happens on the tee'd copy asynchronously.
async fn stream_sse_through(
    upstream_body: Body,
    analysis_tx: mpsc::Sender<Bytes>,
) -> Body {
    let (mut sender, body) = Body::channel();

    tokio::spawn(async move {
        let mut stream = upstream_body;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    // Send to client immediately
                    let _ = sender.send_data(bytes.clone()).await;
                    // Tee to analysis pipeline (non-blocking)
                    let _ = analysis_tx.try_send(bytes);
                }
                Err(e) => {
                    tracing::error!("upstream SSE error: {}", e);
                    break;
                }
            }
        }
    });

    body
}
```

**Key rules:**
- NEVER buffer the full SSE response before forwarding
- Use `Body::channel()` to create a streaming response body
- Tee via `mpsc::Sender` with `try_send` (drop analysis data rather than block the client)
- Parse SSE `data:` lines in the analysis pipeline, not in the forwarding path

### Request Body Tee Without Buffering

For request bodies (which are typically small JSON), tee by reading once and cloning bytes:

```rust
/// Read the request body once, return bytes for both forwarding and analysis.
/// For request bodies (typically < 1MB), full read is acceptable.
/// For large uploads or streaming request bodies, use the chunk-tee pattern instead.
async fn tee_request_body(body: Body) -> Result<(Bytes, Bytes), hyper::Error> {
    let bytes = hyper::body::to_bytes(body).await?;
    Ok((bytes.clone(), bytes))
}
```

For response bodies, ALWAYS use the streaming tee (see SSE pattern above). Never `to_bytes()` a response.

### Protocol Detection

```rust
/// Detect the upstream protocol from the request.
/// Returns the provider and version info needed to:
/// 1. Route to the correct upstream
/// 2. Select the correct response parser
/// 3. Apply provider-specific SSE handling
fn detect_protocol(req: &Request<Body>) -> Protocol {
    let path = req.uri().path();
    let headers = req.headers();

    match () {
        _ if path.starts_with("/proxy/openai/") => Protocol::OpenAI {
            version: extract_api_version(path, "openai"),
            streaming: is_streaming_request(headers),
        },
        _ if path.starts_with("/proxy/anthropic/") => Protocol::Anthropic {
            version: extract_api_version(path, "anthropic"),
            streaming: is_streaming_request(headers),
        },
        _ if path.starts_with("/mcp") => Protocol::MCP {
            transport: detect_mcp_transport(req),
        },
        _ if is_a2a_request(headers) => Protocol::A2A,
        _ => Protocol::Unknown,
    }
}

#[derive(Debug, Clone)]
enum Protocol {
    OpenAI { version: String, streaming: bool },
    Anthropic { version: String, streaming: bool },
    MCP { transport: McpTransport },
    A2A,
    Unknown,
}

#[derive(Debug, Clone)]
enum McpTransport {
    Sse,
    StreamableHttp,
    JsonRpc,
}
```

### Latency Budget

The proxy MUST add < 5ms per request. All heavy processing (PII detection, token counting, DB writes, alert evaluation) happens asynchronously AFTER the response is forwarded.

```
Client Request → [tee body ~0.1ms] → Forward to upstream → [stream response through ~0ms added] → Client
                       ↓                                              ↓
                 [async analysis]                              [async analysis]
                 [async DB write]                              [async DB write]
                 [async alerts]                                [async PII scan]
```

### Fail-Open Design

If analysis, DB writes, or any non-forwarding component fails, the proxy CONTINUES forwarding traffic. Never block or delay the client due to an internal error.

```rust
// CORRECT: fire-and-forget analysis
tokio::spawn(async move {
    if let Err(e) = analyze_request(teed_bytes).await {
        tracing::warn!("analysis failed, traffic unaffected: {}", e);
    }
});

// WRONG: awaiting analysis in the request path
// let analysis = analyze_request(teed_bytes).await?;  // blocks client!
```

### Reference Implementation

Study [agentgateway](https://github.com/agentgateway/agentgateway) for patterns on:
- Rust-based MCP/A2A proxy with SSE transport
- JWT auth and RBAC at the proxy layer
- Session state management for bidirectional protocols
- Kubernetes Gateway API integration

## Dependencies (Cargo.toml)

```toml
[dependencies]
hyper = { version = "1", features = ["full"] }
hyper-util = "0.1"
http-body-util = "0.1"
tokio = { version = "1", features = ["full"] }
bytes = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
reqwest = { version = "0.12", features = ["stream"] }  # for upstream calls
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Checklist for Any Proxy Change

- [ ] Hot path uses hyper directly, not axum
- [ ] SSE responses stream through without buffering
- [ ] Request body tee uses clone, not double-read
- [ ] All analysis is async/fire-and-forget from the forwarding path
- [ ] Fail-open: internal errors never block client traffic
- [ ] Latency added < 5ms (benchmark with `criterion`)
- [ ] `detect_protocol()` updated if new provider added
