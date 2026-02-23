# Changelog

All notable changes to Govrix Platform will be documented in this file.

## [Unreleased]

### Fixed
- mTLS proxy on port 4443 now forwards all AI traffic to the Scout proxy (port 4000) instead of serving a health stub only
- `PlatformConfig::load()` now respects `GOVRIX__PLATFORM__*` environment variable overrides (Figment `Env` layer)
- `RUST_LOG` env var is now honored and takes priority over `GOVRIX_LOG_LEVEL` in log filter chain
- `GET /api/v1/platform/license` now returns full feature set: `compliance_enabled`, `a2a_identity_enabled`, `retention_days`
- K8s manifests now inject `AGENTMESH_DATABASE_URL` and `AGENTMESH_API_KEY` from secrets
- Dockerfile now exposes port 4443 (mTLS proxy)

### Added
- `GET /health` unauthenticated endpoint for K8s liveness/readiness probes (no API key required)
- `POST /api/v1/tenants` endpoint documented in README and DEPLOYMENT.md
- `POST /api/v1/certs/issue` endpoint documented in README
- `AGENTMESH_API_KEY` bearer token auth documented in DEPLOYMENT.md and DEVELOPMENT.md
- `config/policies.example.yaml` referenced in README
- `govrix-keygen` crate listed in README architecture section

## [0.1.0] - 2026-02-19

### Added
- govrix-common: PlatformConfig, LicenseTier (Community/Starter/Growth/Enterprise), TenantRegistry
- govrix-policy: PolicyEngine with YAML rule loading and hot-reload, PII masking, BudgetTracker
- govrix-policy: GovrixPolicyHook bridging Scout's PolicyHook trait to Govrix engine
- govrix-identity: Certificate Authority generation, agent mTLS cert issuance, MtlsConfig
- govrix-server: Full startup pipeline with license validation, budget wiring, mTLS TLS listener
- govrix-server: REST API (7 endpoints: health, license, policies, reload, tenants, certs/issue)
- govrix-keygen: CLI binary to mint and validate license keys
- Tier-based budget defaults (Starter: 50M tokens/$500, Growth: 500M/$5000, Enterprise: unlimited)
- Per-agent token and cost budget limits via config
- Docker and docker-compose support (ports 4000/4001/4443)
- Kubernetes manifests with kustomization
- GitHub Actions CI
