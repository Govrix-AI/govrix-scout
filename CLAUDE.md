# Govrix Scout — OSS AI Agent Observability Proxy

## FIRST: Read Shared Context

**Every session MUST read these before any work:**
1. `.context/MEMORY.md` — Stable project knowledge (schemas, API contracts, architecture)
2. `.context/SESSION_LOG.md` — What was done, what's left, blockers (append-only)

**After every session, append to `.context/SESSION_LOG.md`** with: what was done, what's left, test counts, blockers.

---

## What This Is

Govrix Scout is the **open-source core** of Govrix: a transparent HTTP proxy that sits between AI agents and their APIs (OpenAI, Anthropic, MCP). It captures every request/response for audit, compliance, and cost tracking — with zero agent code changes.

The enterprise features (policy enforcement, mTLS, session recorder, SSO) live in the separate `govrix` repo.

---

## Quick Start

```bash
# First-time setup
./scripts/setup.sh

# Start full stack (TimescaleDB + proxy + dashboard)
make docker-up

# Point agents (one env var, no code changes needed)
export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1
export ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1

# Dashboard
open http://localhost:3000

# Verify
curl http://localhost:4001/health   # {"status":"ok","version":"0.1.0"}
```

---

## Dev Commands

```bash
make setup           # First-time: Rust toolchain + pnpm + deps
make docker-up       # Start TimescaleDB + proxy + dashboard
make docker-down     # Stop containers
make dev-proxy       # Proxy in watch mode — ports 4000/4001 (binary: govrix-scout)
make dev-dashboard   # React dev server with HMR — port 3000
make test            # All Rust tests
make test-proxy      # Proxy crate only (-p govrix-scout-proxy)
make lint            # cargo clippy --workspace -- -D warnings
make fmt             # cargo fmt --all
make check           # Fast cargo check
make build           # Release build — all crates
make build-proxy     # Release build — govrix-scout-proxy crate only
make migrate         # Apply SQL migrations (needs DATABASE_URL)
make db-reset        # Drop + recreate DB then migrate
make docker-logs     # Tail container logs
make ci              # Full CI: fmt-check + lint + test + build
```

---

## Workspace Structure (9 Rust Crates)

```
govrix-scout/
├── crates/
│   ├── govrix-scout-common/   # Shared types, config, models, protocol parsers
│   ├── govrix-scout-proxy/    # Hot-path proxy + REST API — binary: govrix-scout
│   ├── govrix-scout-store/    # PostgreSQL + TimescaleDB layer (sqlx)
│   ├── govrix-scout-cli/      # CLI — binary: govrix-scout-cli
│   ├── govrix-scout-reports/  # PDF + JSON report generation
│   ├── govrix-common/         # Shared identity/policy types
│   ├── govrix-policy/         # Policy engine + PII detector
│   ├── govrix-identity/       # Identity / auth primitives
│   └── govrix-server/         # Auxiliary mgmt server
├── dashboard/                 # React 18 + TypeScript + Vite + Tailwind CSS
├── docker/                    # docker-compose.yml, Dockerfile, nginx.conf
├── init/                      # SQL migrations (idempotent)
├── config/                    # govrix.default.toml, policies.example.yaml
├── docs/                      # Local-only architecture docs (gitignored)
└── scripts/                   # setup.sh, verify.sh
```

---

## Port Map

| Port | Service |
|------|---------|
| 4000 | Proxy — agent traffic |
| 4001 | Management REST API |
| 3000 | React dashboard |
| 5432 | PostgreSQL / TimescaleDB |
| 9090 | Prometheus metrics |

---

## Architecture: Key Invariants

**Hot path** (`govrix-scout-proxy`):
- Uses `hyper` directly — NOT axum — for <1ms p50, <5ms p99 latency
- Management API uses `axum` on port 4001 (separate from hot path)
- Fire-and-forget: events go to a bounded `mpsc` channel (10K) and are never awaited
- **Fail-open**: internal errors must never block upstream agent traffic

**Compliance invariant** — every captured event MUST have:
- `session_id` — groups related requests in a conversation
- `timestamp` — UTC ISO-8601, microsecond precision
- `lineage_hash` — SHA-256 Merkle chain (tamper-evident)
- `compliance_tag` — `pass:all`, `warn:pii_email`, `audit:budget`, etc.

---

## API Endpoints (port 4001)

| Group | Endpoints |
|-------|-----------|
| Health | `GET /health`, `GET /ready`, `GET /metrics` |
| Events | `GET /api/v1/events`, `GET /api/v1/events/{id}`, `GET /api/v1/events/sessions/{session_id}`, `GET /api/v1/events/stream` |
| Agents | `GET /api/v1/agents`, `GET /api/v1/agents/{id}`, `PUT /api/v1/agents/{id}`, `POST /api/v1/agents/{id}/retire`, `GET /api/v1/agents/{id}/events` |
| Costs | `GET /api/v1/costs/summary`, `GET /api/v1/costs/breakdown` |
| Reports | `GET /api/v1/reports/types`, `POST /api/v1/reports/generate`, `GET /api/v1/reports` |
| Config | `GET /api/v1/config` |
| Alerts | `GET /api/v1/alerts`, `GET /api/v1/alerts/{id}`, `GET /api/v1/alerts/stream`, `POST /api/v1/alerts/{id}/acknowledge` |

Bearer token auth on `/api/v1/*`. Response format: `{"data": [...], "total": N}`.

---

## Configuration

```bash
GOVRIX_STORE__DATABASE_URL=postgresql://govrix:govrix@localhost:5432/govrix
GOVRIX_PROXY__LISTEN_PORT=4000
GOVRIX_API_KEY=your_secret_key_here
RUST_LOG=govrix_scout_proxy=info
```

---

## NEVER Commit to OSS Repo

- **NO context files, session logs, or strategy docs** in govrix-scout ever
- The `docs/` folder is **gitignored** — local architecture/teaching docs only, never committed
- This repo is public — anything committed here is visible to everyone

## Scale tiers (documented in docs/architecture.html)

| Tier | Topology | Req/s | Events/day |
|------|----------|-------|------------|
| 1 (default OSS) | Single proxy + TimescaleDB | ~5,000 | ~10M |
| 2 (OSS optional, feature `redis-sink`) | N proxies + Redis Stream | ~15,000 | ~50M |
| 3 (enterprise) | Redpanda + ClickHouse | unbounded | 1B+ |

Anomaly v2 runs post-flush in `flush_batch` (zero hot-path cost). Cold-start SQL seeds EWMA + t-digest from last 24h.

---

## Code Standards

- Rust: `clippy` + `rustfmt` — no warnings
- TypeScript: ESLint + Prettier
- Tests: `cargo test --workspace`
- No hardcoded secrets
- All 4 compliance fields on every event — never optional
