# Govrix Scout — Product Status & Strategic Backlog
Last updated: 2026-03-01

This is the reference document for the next implementation session.
It reflects a full codebase audit conducted against a 17-page external due diligence report.

---

## What Govrix Scout Is

Rust-based OSS HTTP proxy that sits between AI agents and LLM providers (OpenAI, Anthropic).
Intercepts every request/response for audit, cost tracking, and compliance.

Open-core model: Scout OSS (free) → Enterprise (paid).

- Hot path: `hyper` on port 4000 (<1ms p50)
- Management API: `axum` on port 4001
- Database: PostgreSQL 16 + TimescaleDB
- Dashboard: React 18 + Vite + Tailwind

---

## Feature Audit — Current Status

| Feature | Status | Location | Notes |
|---|---|---|---|
| PII Detection | IMPLEMENTED | `crates/govrix-scout-proxy/src/policy/pii.rs` | Regex, 5 types (email, phone, SSN, credit card, IPv4). Detection + tagging only — does NOT mask. Never stores PII values. 25+ tests pass. |
| Cost Tracking | IMPLEMENTED | `crates/govrix-scout-store/src/costs.rs` | Real DB queries. `get_cost_summary()`, `get_cost_breakdown()`, `get_cost_timeseries()`. Uses `cost_daily` materialized view. P99 latency percentiles. |
| Agent Registry | IMPLEMENTED | `crates/govrix-scout-store/src/agents.rs` | Auto-populated from traffic via `upsert_agent()`. Tracks total_requests, tokens, cost, first/last_seen, status, fingerprint. Full CRUD. |
| Compliance Fields | IMPLEMENTED | `crates/govrix-scout-common/src/models/event.rs` | All 4 mandatory fields always set: session_id, timestamp, lineage_hash, compliance_tag. Compile-time enforced in `AgentEvent::new()`. |
| SSE Streaming | IMPLEMENTED | `crates/govrix-scout-proxy/src/proxy/streaming.rs` | True pass-through (non-blocking tee). Client gets chunks immediately. Detects OpenAI `[DONE]` and Anthropic `message_stop`. 11 tests pass. |
| YAML Policy Engine | IMPLEMENTED | `crates/govrix-policy/src/engine.rs` | Full rule evaluation. Operators: equals, not_equals, contains, greater_than, less_than, matches (regex). Fields: model, agent_id, provider, cost_usd, tokens. Actions: Allow/Block/Alert. 16 tests pass. |
| Budget Enforcement | IMPLEMENTED (limited) | `crates/govrix-scout-proxy/src/policy/budget.rs` | Per-agent + global daily caps. 90% utilization alert. UTC midnight reset. CRITICAL GAP: in-memory only — resets on service restart. |
| Session Forensics | STUB (dead code) | `crates/govrix-policy/src/session.rs` | `SessionRecording` struct + SHA-256 integrity verification + markdown rendering exist. NEVER CALLED from hot path. `#[allow(dead_code)]` on module. |
| mTLS | STUB (not wired) | `crates/govrix-identity/src/mtls.rs` | `MtlsConfig` struct + CA generation exist. NOT integrated into proxy handler. No TLS validation on incoming requests. |
| Kubernetes | STUB (manifests only) | `k8s/*.yaml` | Basic deployment/service/configmap/secret/postgres YAMLs exist. No sidecar injector, no HA, no persistent volumes, no init containers for DB init. |
| Reports | STUB (API only) | `crates/govrix-scout-proxy/src/api/handlers/reports.rs` | Returns HTTP 202 + "available in Govrix Scout SaaS" message. No actual generation. 4 types listed: usage_summary, cost_breakdown, agent_inventory, activity_log. |
| Kill Switch | MISSING | — | `retire_agent()` exists (sets status=blocked) but agent.status is never checked in hot-path request handler. No global stop. No `POST /api/v1/kill-switch`. |
| Multi-provider routing | MISSING | — | Only OpenAI and Anthropic parsers. No Bedrock, Azure OpenAI, VertexAI, Cohere, Gemini. |
| Webhook connectors | MISSING | — | No push to Datadog, Splunk, PagerDuty, Vanta. |
| OIDC/SSO | MISSING (enterprise only) | — | Basic SSO locked behind enterprise paywall — "SSO Tax" problem. |

---

## Strategic Analysis from Due Diligence Report

### What the Report Got RIGHT (verified accurate)

**1. Rust vs LiteLLM performance moat**
LiteLLM P99 degrades to 90s at 500 RPS, 3-4s cold start, memory bloat. Verified in production reports.
Rust is a genuine, durable technical moat for any team running at scale.

**2. Semantic gap / eBPF threat**
The proxy only sees LLM API traffic. It misses: internal tool execution, DB calls, file system access,
lateral movement within agent orchestration. eBPF tools (AgentSight, Eunomia) solve this at the kernel level.
This is a real long-term competitive threat with approximately a 24-month window.

**3. SSO Tax**
Locking basic SSO behind the enterprise paywall causes products to be banned from internal corporate networks
during security reviews. CISOs require SSO as a baseline condition, not a premium feature.

**4. Kill switch missing**
Correct. `retire_agent()` sets `status=blocked` in the database, but the hot-path interceptor never
reads agent status before forwarding requests. A "blocked" agent continues to pass traffic.

**5. Cost control as GTM wedge**
"Stop my $5k weekend AWS bill" beats "compliance" as a sales entry point. Correct framing.
Cost observability is the hook; governance and compliance are the upsell.

**6. Open-core model is sound**
Scout free → Enterprise paid. Observability shows the problem; governance provides the solution.
Correct positioning for a developer-first product.

**7. First customers: Series B/C SaaS, not banks**
Banks and regulated financial institutions require SOC2 Type II certification before signing contracts.
Start with growth-stage SaaS companies that have AI agent sprawl problems and no audit trail.

**8. EU AI Act August 2026 deadline**
Real and verified. High-risk AI systems must have audit trails by August 2026.
This creates urgency for any EU-facing SaaS company deploying AI agents.

### What the Report Got WRONG (about our implementation)

**1. "Automated Compliance Reporting (SOC2, HIPAA)" warning**
We never built this. The reports API returns HTTP 202 with a "coming soon" message.
The report's warning about compliance report accuracy doesn't apply to stubs.

**2. "PII Masking trap"**
The report conflates PII masking (which breaks agent reasoning by corrupting context windows)
with PII detection (which is architecturally correct). We detect and tag — we do not mask.
Our approach is the right one. The warning does not apply.

**3. "YAML Policy Engine insufficient"**
True that YAML-only policies can be bypassed by polymorphic prompt injections.
But the report frames the policy engine as absent or trivial. We have a working engine with
6 operators, 5 field types, 3 actions, and 16 passing tests. The gap is sophistication, not existence.

**4. "Budget enforcement missing"**
We have budget enforcement. The gap is persistence (counters reset on restart), not existence.
This is a real bug, not a missing feature.

**5. "eBPF is immediate / non-negotiable"**
Real long-term threat. Not a sprint item for a small team. 12-18 month roadmap, post-revenue.
The report overstates urgency. No enterprise customer will block on eBPF absence in 2026.

---

## Prioritized Implementation Backlog

### Priority 1 — Fix Built-But-Broken Features
High value, low effort. These features exist but are not wired together.

**P1-A: Wire session forensics into hot path**
- What: `session.rs` is complete but dead code. Connect it to the request/response cycle.
- Why: Session replay is a major differentiator for debugging non-deterministic LLM behavior.
  Customers can replay exactly what an agent did and why a decision was made.
- File to modify: `crates/govrix-policy/src/session.rs`
- Integrate into: `crates/govrix-scout-proxy/src/proxy/interceptor.rs`
- Effort: 1-2 days

**P1-B: Persist budget counters across restarts**
- What: Daily token/cost counters reset on service restart (in-memory HashMap only).
  Write counters to DB on update; reload on startup.
- Why: Makes the hard-cap promise real. Currently any restart bypasses all budget limits.
- File to modify: `crates/govrix-scout-proxy/src/policy/budget.rs`
- DB option: Add `budget_daily` table or store in existing `cost_daily` materialized view.
- Effort: 1 day

**P1-C: Wire kill switch into hot-path request handler**
- What: `retire_agent()` sets `status=blocked` but the hot path never reads agent status.
  Add an agent status check before forwarding any request.
- Why: Critical safety feature. A "blocked" agent currently continues to pass all traffic.
- File to modify: `crates/govrix-scout-proxy/src/proxy/interceptor.rs`
  — Add agent status lookup (from store or in-memory cache) before proxying
- Also add: `POST /api/v1/agents/{id}/block` and `POST /api/v1/agents/{id}/unblock` endpoints
- Also add: `POST /api/v1/kill-switch` (global emergency stop)
- Effort: 1 day

**P1-D: Fix 38 dead_code Clippy warnings**
- What: Clean up `#[allow(dead_code)]` annotations by either wiring up the code (preferred)
  or removing it if it is genuinely unused.
- Why: Clippy hygiene signals production readiness to enterprise buyers and open-source contributors.
- Effort: 0.5 days

---

### Priority 2 — Real Competitive Gaps
Medium effort, high strategic value. Required for enterprise sales.

**P2-A: Multi-provider routing (Bedrock, Azure OpenAI, VertexAI)**
- What: Add request/response parsers for AWS Bedrock, Azure OpenAI, Google VertexAI.
- Why: The "multi-cloud control plane" story requires supporting all major providers.
  Currently only OpenAI and Anthropic are parsed; all others pass through without cost/token tracking.
- File to add: `crates/govrix-scout-common/src/parsers/` — new provider-specific parsers
- Effort: 3-5 days per provider

**P2-B: Webhook connectors (Datadog, Splunk, PagerDuty)**
- What: Instead of building compliance reports, push events to existing SIEM/observability stacks.
- Why: Correct architecture per the due diligence report. Enterprises already have Datadog or Splunk.
  Be a telemetry source, not a competing reporting tool.
- Add: `POST /api/v1/webhooks` endpoint for connector configuration
- Add: Background webhook dispatcher that fans out events from the event pipeline
- Effort: 3-4 days

**P2-C: Move basic OIDC/SSO to OSS tier**
- What: Move Google Workspace OAuth and GitHub Teams SSO out of the enterprise paywall into Scout OSS.
- Why: Eliminates the SSO Tax. Products behind an SSO paywall get blocked by IT/security during evaluations.
- Keep behind Enterprise: Granular RBAC, custom IdP federation (Okta/AD/LDAP), cryptographically signed audit logs.
- Effort: 2-3 days

**P2-D: Wire mTLS into proxy handler**
- What: `mtls.rs` config and CA generation exist. Connect them to actual TLS validation
  on incoming connections in the proxy main loop.
- Why: Required to pass CISO security review for any real enterprise deployment.
  Without mTLS, the "zero-trust" claim is paper-only.
- File to modify: `crates/govrix-identity/src/mtls.rs`
- Wire into: `crates/govrix-scout-proxy/src/main.rs`
- Effort: 2-3 days

**P2-E: Production-grade Kubernetes manifests**
- What: Sidecar injection pattern, PersistentVolumeClaims for Postgres, resource limits/requests,
  init containers for DB schema initialization, readiness/liveness probes.
- Why: Existing `k8s/` manifests are development-grade. Enterprise infrastructure teams will
  immediately reject manifests that lack resource limits and persistent volumes.
- Effort: 3-4 days

---

### Priority 3 — Post-Revenue Roadmap

**P3-A: eBPF sidecar agent**
- What: Linux kernel-level observability to close the semantic gap.
  Reference: AgentSight on GitHub (eunomia-bpf/agentsight).
- Why: Long-term existential threat from eBPF-based security tools (Protect AI, AgentSight).
  The proxy only sees LLM API calls. eBPF sees tool execution, file I/O, syscalls, lateral movement.
- Requires: Linux 5.8+, CAP_BPF, significant systems engineering investment.
- Effort: 2-3 months for MVP
- Timeline: 12-18 months, post first enterprise contract

**P3-B: SLM guardrails (ONNX runtime in Rust)**
- What: Replace YAML-only policies with local small language models for semantic policy evaluation.
- Why: Regex and YAML policies can be bypassed by polymorphic prompt injections.
  Aporia achieves 98% detection at 0.34s latency using SLMs. This is the right long-term direction.
- Effort: 1-2 months
- Timeline: Post-revenue, once policy engine gaps become a real sales blocker

**P3-C: AIDIP/AGP agent registry compliance**
- What: Make the agent registry conform to the IETF AI Agent Discovery and Invocation Protocol.
- Why: Future interoperability with LangChain, AutoGen, CrewAI, Semantic Kernel agent frameworks.
- Wait for: Standards to stabilize (still drafts as of 2026-03).
- Effort: 2-3 weeks once specifications stabilize

---

## Competitive Landscape (verified 2026)

| Competitor | Type | Weakness vs Govrix |
|---|---|---|
| LiteLLM | OSS LLM proxy | Python: 90s P99 at 500 RPS, 3-4s cold start, memory bloat |
| Langfuse | Observability SDK | Requires code instrumentation, no proxy interception |
| Helicone | Proxy observability | Python-based, limited policy enforcement |
| Portkey | AI gateway | 250+ models but no compliance or audit trail |
| Bifrost | Go proxy | Fast but minimal governance features |
| Kong/Tyk | API gateway | No LLM semantic understanding |
| AWS Bedrock | Native guardrails | Vendor lock-in, only works within AWS VPC |
| Azure AI Foundry | Native guardrails | Vendor lock-in, only works with Azure/OpenAI |

**Govrix moat**: Rust performance + compliance-first architecture + open-core model + multi-cloud independence.

---

## Go-to-Market Notes

- Entry point: Cost control ("stop my $5k weekend bill"), not compliance
- First customers: Series B/C SaaS with AI agent sprawl, NOT regulated banks
- EU AI Act deadline: August 2026 — creates urgency for EU-facing SaaS companies
- SSO Tax fix (P2-C) is a prerequisite for any enterprise evaluation to succeed
- Kill switch (P1-C) is required for any CISO to approve production deployment
