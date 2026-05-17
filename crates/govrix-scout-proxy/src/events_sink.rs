//! Stage 4: pluggable event sinks.
//!
//! The proxy hot path used to hold a concrete `EventSender` (an mpsc
//! channel). For Tier-2 multi-replica deployments we want to swap that for a
//! Redis Streams sink (or any other transport) without rewriting the
//! interceptor. This module introduces the [`EventSink`] trait, the default
//! in-process [`MpscSink`], and a feature-gated [`RedisStreamSink`] stub.
//!
//! Hot-path invariants preserved:
//! - `EventSink::send` is **sync** and **non-blocking** — callers cannot
//!   await on the interceptor path.
//! - Errors are logged and dropped (fail-open).
//!
//! Implementation note (Stage 4 commit): we chose a *sibling module*
//! (`events_sink.rs`) rather than turning `events.rs` into a directory
//! module. This keeps the diff small — none of the existing event channel
//! plumbing has to move — and the trait sits in its own logical home.

use std::sync::Arc;

use govrix_scout_common::models::event::AgentEvent;

use crate::events::EventSender;

/// Abstract event sink. Implementations must be non-blocking on `send`.
///
/// Hot path callers do:
/// ```ignore
/// let sink: Arc<dyn EventSink> = ...;
/// sink.send(event); // fire-and-forget, never awaits
/// ```
#[async_trait::async_trait]
pub trait EventSink: Send + Sync {
    /// Best-effort send. Implementations MUST NOT block — fire-and-forget.
    /// Errors must be swallowed and logged (fail-open).
    fn send(&self, event: AgentEvent);

    /// Approximate channel depth (for metrics). Returns 0 if unknown.
    fn depth(&self) -> usize {
        0
    }
}

// ── Default in-process sink ──────────────────────────────────────────────────

/// In-process mpsc sink — the default. Wraps an [`EventSender`] (which
/// internally is a `tokio::sync::mpsc::Sender` plus metrics).
///
/// `EventSender` is itself cheaply `Clone`able, so this newtype holds it by
/// value rather than `Arc<EventSender>`; we still share the sink across the
/// hot path via `Arc<dyn EventSink>`.
pub struct MpscSink {
    inner: EventSender,
}

impl MpscSink {
    pub fn new(sender: EventSender) -> Self {
        Self { inner: sender }
    }

    /// Access the underlying `EventSender` (used by `main.rs` to keep the
    /// existing channel-construction flow intact).
    pub fn sender(&self) -> &EventSender {
        &self.inner
    }
}

#[async_trait::async_trait]
impl EventSink for MpscSink {
    fn send(&self, event: AgentEvent) {
        self.inner.send(event);
    }

    fn depth(&self) -> usize {
        self.inner.approx_depth()
    }
}

/// Convenience constructor: wrap an [`EventSender`] in an `Arc<dyn EventSink>`.
pub fn mpsc_sink(sender: EventSender) -> Arc<dyn EventSink> {
    Arc::new(MpscSink::new(sender))
}

// ── Tier-2 Redis Streams sink (stub, feature-gated) ──────────────────────────

#[cfg(feature = "redis-sink")]
pub use redis_sink::RedisStreamSink;

#[cfg(feature = "redis-sink")]
mod redis_sink {
    use super::*;
    use redis::aio::ConnectionManager;

    /// Tier-2 sink that pushes each event onto a Redis Stream
    /// (`govrix:events`) via `XADD`. Multi-replica deployments point all
    /// proxy replicas at the same Redis instance and have downstream
    /// consumers read with `XREADGROUP`.
    ///
    /// **Stub status**: the actual `XADD` is a `todo!()` — the goal at this
    /// stage is to prove the trait abstraction holds and the crate compiles
    /// with the feature flag both ON and OFF.
    pub struct RedisStreamSink {
        #[allow(dead_code)]
        conn: ConnectionManager,
    }

    impl RedisStreamSink {
        pub fn new(conn: ConnectionManager) -> Self {
            Self { conn }
        }
    }

    #[async_trait::async_trait]
    impl EventSink for RedisStreamSink {
        fn send(&self, event: AgentEvent) {
            // Fire-and-forget: spawn a task so the hot path never awaits.
            let conn = self.conn.clone();
            tokio::spawn(async move {
                let _ = conn; // silence unused warning until implemented
                let _payload = match serde_json::to_string(&event) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(error = %e, "redis sink: failed to serialize event (dropping)");
                        return;
                    }
                };
                // TODO(stage-4.3): XADD govrix:events * payload <_payload>.
                // Stub: panic if anyone actually wires this up in prod.
                todo!("RedisStreamSink::send XADD not yet implemented")
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::create_channel;

    #[test]
    fn mpsc_sink_send_is_non_blocking() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (sender, mut rx) = create_channel();
            let sink: Arc<dyn EventSink> = mpsc_sink(sender);

            let ev = govrix_scout_common::models::event::AgentEvent::new(
                "agent-1",
                uuid::Uuid::now_v7(),
                govrix_scout_common::models::event::EventDirection::Outbound,
                "POST",
                "https://api.openai.com/v1/chat/completions",
                govrix_scout_common::models::event::Provider::OpenAI,
                "genesis",
                "audit:none",
            );

            sink.send(ev);
            assert!(rx.recv().await.is_some());
            // depth is approximate; just confirm the trait call works.
            let _ = sink.depth();
        });
    }
}
