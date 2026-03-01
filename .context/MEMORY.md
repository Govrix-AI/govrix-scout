# Govrix Scout — Stable Project Knowledge

## What It Is
Rust-based OSS HTTP proxy between AI agents and LLM providers (OpenAI, Anthropic).
Intercepts every request/response for audit, cost tracking, and compliance.
Open-core: Scout OSS (free) → Enterprise (paid).

## Architecture
- Hot path: `hyper` on port 4000 (<1ms p50, <5ms p99) — NEVER use axum here
- Management API: `axum` on port 4001 (17 endpoints, bearer auth)
- Database: PostgreSQL 16 + TimescaleDB hypertable for events
- ORM: `sqlx` 0.8 with `runtime-tokio-rustls`
- Dashboard: React 18 + Vite + Tailwind + TanStack Query (5s polling) + Recharts
- Port 3000: React dashboard | 5432: Postgres | 9090: Prometheus | 4443: mTLS A2A (Enterprise)

## Workspace Crates (OSS)
- `govrix-scout-common` — shared types, models, parsers
- `govrix-scout-store` — DB layer (sqlx), agents, costs
- `govrix-scout-proxy` — hot-path proxy binary (`govrix-scout`)
- `govrix-scout-cli` — CLI binary (`govrix-scout-cli`)
- `govrix-scout-reports` — report stubs

## Compliance Invariant
Every `AgentEvent` MUST have (compile-time enforced in `AgentEvent::new()`):
- `session_id`, `timestamp`, `lineage_hash` (SHA-256 Merkle), `compliance_tag`

## Event Pipeline
Fire-and-forget: bounded mpsc channel (10K) → background batch writer (100ms / 100 events) → PostgreSQL
Event writes are NEVER awaited in the hot-path request handler.

## Key Source Locations
| Feature | File |
|---|---|
| PII detection | `crates/govrix-scout-proxy/src/policy/pii.rs` |
| Budget enforcement | `crates/govrix-scout-proxy/src/policy/budget.rs` |
| SSE streaming | `crates/govrix-scout-proxy/src/proxy/streaming.rs` |
| Hot-path interceptor | `crates/govrix-scout-proxy/src/proxy/interceptor.rs` |
| Agent registry | `crates/govrix-scout-store/src/agents.rs` |
| Cost queries | `crates/govrix-scout-store/src/costs.rs` |
| Event model | `crates/govrix-scout-common/src/models/event.rs` |
| Session forensics | `crates/govrix-policy/src/session.rs` (stub — dead code) |
| mTLS config | `crates/govrix-identity/src/mtls.rs` (stub — not wired) |
| YAML policy engine | `crates/govrix-policy/src/engine.rs` |

## Feature Status Summary
- WORKING: PII detection, cost tracking, agent registry, compliance fields, SSE streaming, YAML policy engine
- BROKEN (built but not wired): session forensics, mTLS, kill switch (retire_agent exists; status never checked in hot path), budget persistence (in-memory only — resets on restart)
- MISSING: multi-provider routing (no Bedrock/Azure/VertexAI), webhook connectors, OIDC/SSO
- STUBS: reports API (returns HTTP 202), Kubernetes manifests (dev-grade only)

## Next Implementation Priority
1. P1-C: Wire kill switch — add agent status check in `interceptor.rs` before forwarding
2. P1-B: Persist budget counters — write daily counters to DB; restart safety
3. P1-A: Wire session forensics — connect `session.rs` into `interceptor.rs`
4. P1-D: Fix 38 dead_code Clippy warnings
5. P2-A: Multi-provider routing (Bedrock, Azure OpenAI, VertexAI)
6. P2-B: Webhook connectors (Datadog, Splunk, PagerDuty)
7. P2-C: Move basic OIDC/SSO to OSS tier (fix SSO Tax)
8. P2-D: Wire mTLS into proxy handler
9. P2-E: Production-grade Kubernetes manifests

## Docker / Build Notes
- Local builds: `Dockerfile.prebuilt` + `docker-compose.local.yml`
- Build Linux arm64 binary inside Docker with host cargo cache — macOS binary (Mach-O) won't run in Linux container
- Axum 0.8: route params MUST use `{param}` syntax, NOT `:param`
- Healthcheck URLs: use `127.0.0.1` not `localhost` (IPv6 resolution issue in Alpine)
- `cargo fetch --locked` MUST precede `cargo build` in Dockerfile

## Session Context Files (read every session)
- `.context/MEMORY.md` — this file (stable knowledge)
- `.context/SESSION_LOG.md` — append-only session log
- `docs/PRODUCT_STATUS.md` — full feature audit + strategic backlog
