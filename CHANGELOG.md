# Changelog

All notable changes to Govrix Scout will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.2.0] - 2026-03-21

### Added
- Dashboard: 18-page React 18 + TypeScript + Vite rewrite (replaced Next.js 14)
  - New pages: budgets, compliance, EU AI Act, kill switch, PII activity, policies, risk overview, sessions, traces, stream
- TanStack Query v5 integration for auto-refreshing data fetching
- Tailwind CSS 3 + Recharts for responsive charts and UI
- Budget enforcement system (daily/monthly limits per agent or project)
- Session tracking to group related requests in conversations
- Distributed tracing for multi-step agent workflow debugging
- SQL migrations 006–011: budget_daily, budget_config, projects, agent_tracing, sessions, traces
- Policy engine: YAML-based rules to allow, block, or alert on agent behaviors
- Streaming support: full SSE and chunked transfer with <5ms p99 overhead
- REST API endpoints for events, agents, costs, reports, and config (port 4001)
- Install scripts for Linux/macOS (`install.sh`) and Windows (`install.ps1`)
- Docker multi-stage builds: Dockerfile, Dockerfile.dashboard (node + nginx), Dockerfile.prebuilt
- Docker Compose variants: development, local, production
- Kubernetes manifests with Kustomize support
- GitHub Actions: release workflow (Docker image push to GHCR), security scanning workflow

### Changed
- Dashboard rewritten from Next.js 14 to React 18 + Vite + TypeScript
- README overhauled with full architecture docs, database schema, API reference, and expanded integration examples
- Logo moved from repo root to `docs/assets/logo.png`

### Removed
- `remotion/` directory — orphaned video generation project, not integrated with the product
- `proxy_logs.txt` — empty log file artifact
- `imgonline-com-ua-GIF-animation-2aVXJx6VtOK.gif` — unused demo GIF from repo root

## [0.1.0] - 2026-02-19

### Added
- govrix-scout-proxy: HTTP proxy interceptor with PolicyHook extension point
- govrix-scout-store: PostgreSQL event persistence layer
- govrix-scout-common: Shared types (AgentEvent, Config, Provider enum)
- govrix-scout-cli: CLI with `status`, `agents list`, `events list` subcommands
- govrix-scout-reports: UsageSummary, CostBreakdown, AgentInventory, ActivityLog reports
- govrix-scout-reports: HTML output with inline SVG bar charts
- Prometheus metrics endpoint at /metrics
- Dashboard: Next.js 14 web UI (overview, agents, events pages)
- Docker and docker-compose support
- Kubernetes manifests (namespace, configmap, deployment, service, postgres)
- GitHub Actions CI (test + clippy on PR and main push)
- Scout diagnose mode: detects governance gaps without blocking traffic
