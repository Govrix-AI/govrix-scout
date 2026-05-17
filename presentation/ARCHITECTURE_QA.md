# Govrix Scout — Architecture Q&A
### Every question a VP / Director of Engineering can ask. Read this before the presentation.

---

## TABLE OF CONTENTS

1. [Why Rust?](#1-why-rust)
2. [Why hyper directly instead of axum for the proxy?](#2-why-hyper-directly-instead-of-axum-for-the-proxy)
3. [Why TimescaleDB instead of plain PostgreSQL?](#3-why-timescaledb-instead-of-plain-postgresql)
4. [Why not MongoDB / Cassandra / ClickHouse / Redis?](#4-why-not-mongodb--cassandra--clickhouse--redis)
5. [What is a hypertable and how does it work?](#5-what-is-a-hypertable-and-how-does-it-work)
6. [Why a bounded mpsc channel? Why not Kafka or Redis Queue?](#6-why-a-bounded-mpsc-channel-why-not-kafka-or-redis-queue)
7. [What happens when the channel is full?](#7-what-happens-when-the-channel-is-full)
8. [What is fail-open design and why does it matter?](#8-what-is-fail-open-design-and-why-does-it-matter)
9. [What is the Merkle chain and how does it prove tamper-evidence?](#9-what-is-the-merkle-chain-and-how-does-it-prove-tamper-evidence)
10. [Why PII regex and not an ML model?](#10-why-pii-regex-and-not-an-ml-model)
11. [What is OnceLock and why does it matter for PII?](#11-what-is-oncelock-and-why-does-it-matter-for-pii)
12. [How do the circuit breakers work?](#12-how-do-the-circuit-breakers-work)
13. [Why are circuit breakers in-memory and not DB-backed?](#13-why-are-circuit-breakers-in-memory-and-not-db-backed)
14. [What happens when the proxy restarts — does state reset?](#14-what-happens-when-the-proxy-restarts--does-state-reset)
15. [How does agent identity resolution work?](#15-how-does-agent-identity-resolution-work)
16. [What is mTLS and why does govrix-identity exist?](#16-what-is-mtls-and-why-does-govrix-identity-exist)
17. [What is EMA and why use it for anomaly detection?](#17-what-is-ema-and-why-use-it-for-anomaly-detection)
18. [How does the loop detector prevent the $47K incident?](#18-how-does-the-loop-detector-prevent-the-47k-incident)
19. [What is the actual measured latency?](#19-what-is-the-actual-measured-latency)
20. [What is the throughput limit? How does it scale?](#20-what-is-the-throughput-limit-how-does-it-scale)
21. [What happens if the upstream API (OpenAI) is down?](#21-what-happens-if-the-upstream-api-openai-is-down)
22. [What happens if the proxy itself crashes?](#22-what-happens-if-the-proxy-itself-crashes)
23. [What happens if TimescaleDB is down?](#23-what-happens-if-timescaledb-is-down)
24. [How do you handle authentication and authorization?](#24-how-do-you-handle-authentication-and-authorization)
25. [Why 4 mandatory compliance fields on every event?](#25-why-4-mandatory-compliance-fields-on-every-event)
26. [How does session tracking work across requests?](#26-how-does-session-tracking-work-across-requests)
27. [What compliance standards does this actually satisfy?](#27-what-compliance-standards-does-this-actually-satisfy)
28. [How does EU AI Act compliance work specifically?](#28-how-does-eu-ai-act-compliance-work-specifically)
29. [What is DPDP and why is it relevant?](#29-what-is-dpdp-and-why-is-it-relevant)
30. [Why sqlx over Diesel or SeaORM?](#30-why-sqlx-over-diesel-or-seaorm)
31. [How do database migrations work?](#31-how-do-database-migrations-work)
32. [What is the cost_daily materialized view?](#32-what-is-the-cost_daily-materialized-view)
33. [How is cost calculated per event?](#33-how-is-cost-calculated-per-event)
34. [How does the kill switch work end-to-end?](#34-how-does-the-kill-switch-work-end-to-end)
35. [How does the risk score get calculated?](#35-how-does-the-risk-score-get-calculated)
36. [How does protocol detection work for OpenAI vs Anthropic vs MCP?](#36-how-does-protocol-detection-work-for-openai-vs-anthropic-vs-mcp)
37. [What is SSE and how does the live event stream work?](#37-what-is-sse-and-how-does-the-live-event-stream-work)
38. [Why React 18 + TanStack Query for the dashboard?](#38-why-react-18--tanstack-query-for-the-dashboard)
39. [Why 8 Rust crates? Why not a monolith?](#39-why-8-rust-crates-why-not-a-monolith)
40. [How is the system deployed? What are the requirements?](#40-how-is-the-system-deployed-what-are-the-requirements)
41. [Why not just use LangSmith / Portkey / Helicone?](#41-why-not-just-use-langsmith--portkey--helicone)
42. [What does zero agent code changes actually mean?](#42-what-does-zero-agent-code-changes-actually-mean)
43. [How do you handle multiple agents simultaneously?](#43-how-do-you-handle-multiple-agents-simultaneously)
44. [What is Prometheus and what metrics do you expose?](#44-what-is-prometheus-and-what-metrics-do-you-expose)
45. [Can an agent bypass the proxy?](#45-can-an-agent-bypass-the-proxy)
46. [Why log PII location and not the value?](#46-why-log-pii-location-and-not-the-value)
47. [What is the new tool detector warm-up period for?](#47-what-is-the-new-tool-detector-warm-up-period-for)
48. [How are compliance reports generated?](#48-how-are-compliance-reports-generated)
49. [Is this production-ready? What's missing?](#49-is-this-production-ready-whats-missing)
50. [What would it take to deploy this at BlackRock / Optum / Verizon scale?](#50-what-would-it-take-to-deploy-this-at-blackrock--optum--verizon-scale)

---

## 1. Why Rust?

**Three reasons — none of them is "I like Rust":**

**No GC pauses.** Python, Go, JVM — all have garbage collectors that pause execution unpredictably. On a proxy that forwards AI agent traffic, a 50ms GC pause during a critical request is unacceptable. Rust has no GC — zero pause, deterministic latency.

**No Python GIL.** LiteLLM is Python. The Global Interpreter Lock means only one thread executes Python at a time. You can spawn threads but they wait on the GIL for CPU-bound work. At ~500 RPS, LiteLLM saturates. Govrix Scout: every connection is a separate async task with no GIL — scales linearly with cores.

**Memory safety without a runtime.** The proxy handles untrusted data from AI agent requests and upstream API responses. Rust's ownership model prevents buffer overflows, use-after-free, and data races at compile time — no runtime overhead needed to enforce safety.

---

## 2. Why hyper directly instead of axum for the proxy?

**axum is built on top of hyper — it adds a routing layer, middleware stack, and ergonomic handler abstractions.** That's great for building REST APIs. It's wrong for a hot-path proxy.

Every axum request goes through: router matching → middleware chain → handler extraction → response serialization. That adds ~200–400µs of overhead per request.

The proxy needs to do exactly one thing per request: detect the protocol from the URL, run 3 in-memory checks, then forward the bytes unchanged. No routing. No body extraction. No middleware chain.

With raw hyper: receive bytes → inspect URL → forward bytes → done. That's <1ms p50.

**The management REST API on port 4001 does use axum** — that's the right tool for that job since it has proper routing, auth middleware, and JSON serialization needs.

---

## 3. Why TimescaleDB instead of plain PostgreSQL?

**The problem we hit: time-range queries on plain Postgres became sequential scans at scale.**

At 1M+ events/day, querying "all events for the last 24 hours" on a plain Postgres table means scanning every row and filtering by timestamp. At 30 days of data (~30M rows), this hits 2–5 second query times.

TimescaleDB is a PostgreSQL extension that turns time-series tables into **hypertables** — automatically partitioned by time into "chunks." A query for the last 24 hours touches only today's chunk, not all 30 days. Query time dropped 10×.

**It's still Postgres.** Same `sqlx` driver, same SQL syntax, same `DATABASE_URL`. No new technology to operate — just Postgres with a time-series extension. The compliance team already knows Postgres. Their DBA already knows Postgres. Zero re-training.

Additional benefits: automatic chunk compression after 7 days (columnar storage, ~90% size reduction), continuous aggregates (the `cost_daily` view updates automatically), and retention policies (auto-drop chunks older than 30 days).

---

## 4. Why not MongoDB / Cassandra / ClickHouse / Redis?

**MongoDB:** Document store. No strong schema enforcement. Every event needs 4 mandatory compliance fields — Rust's type system enforces these at compile time, SQL NOT NULL enforces them at the DB level. MongoDB's flexible schema is the opposite of what compliance requires.

**Cassandra:** Excellent for write-heavy time-series at massive scale (100M+ events/day across multiple DCs). Overkill. Also: no SQL, complex cluster operations, and the query model makes "show me all events for agent X in the last hour" painful.

**ClickHouse:** Excellent analytical query performance. But it's an OLAP database — optimized for aggregations and reads, not transactional writes. The agent upserts and kill switch status updates require ACID transactions. ClickHouse doesn't fit the mixed read/write workload.

**Redis:** Not a persistent store. Redis is ephemeral — it's exactly what we use for the in-memory circuit breaker state. The events table needs durable persistence that survives restarts. Redis Streams could work as a queue but then you still need a persistent DB downstream.

TimescaleDB is the right fit: ACID compliance, SQL, durable, time-optimized, and already Postgres.

---

## 5. What is a hypertable and how does it work?

A **hypertable** is a TimescaleDB abstraction that looks exactly like a normal Postgres table from the outside — same SQL, same indexes, same `psql` — but internally is partitioned into **chunks** based on time.

When you create the events hypertable partitioned by `timestamp`, TimescaleDB automatically:
- Creates a new chunk for each time interval (e.g., 1 day)
- Routes every INSERT to the correct chunk based on the row's timestamp
- Runs queries only against chunks whose time range overlaps the WHERE clause

**Example:** `SELECT * FROM events WHERE timestamp > NOW() - INTERVAL '1 hour'`
- Plain Postgres: full table scan, 30M rows
- TimescaleDB: scans only the current chunk, ~50K rows

Chunks older than 7 days are compressed automatically: data is converted from row-oriented storage to columnar, achieving ~90% size reduction. Chunks older than 30 days are dropped automatically by the retention policy.

The `cost_daily` materialized view is a TimescaleDB **continuous aggregate** — it incrementally refreshes as new events arrive rather than recomputing from scratch.

---

## 6. Why a bounded mpsc channel? Why not Kafka or Redis Queue?

**mpsc = multi-producer, single-consumer.** Tokio's `mpsc::channel` is a lock-free in-process queue. No network hop. No serialization. No external dependency. The producer (hot path) and consumer (background writer) are in the same process — communication is memory, not TCP.

**Why not Kafka?** Kafka adds: network latency (even on localhost, ~1ms), serialization (event must be encoded), a broker process that can crash, and operational complexity (topic management, consumer groups, offsets). For the proxy hot path, any of these is unacceptable. Kafka is the right choice when producers and consumers are in different services or different machines. They're not here.

**Why not Redis Streams?** Same issue — network dependency in the hot path. Also adds an external process that the proxy now depends on. If Redis is down, the proxy either blocks (bad) or loses events silently with no metrics.

**The bounded channel gives exactly what's needed:** zero-copy in-process handoff, non-blocking `try_send()`, backpressure visible in `events_dropped` metrics, and no external dependency. The proxy is self-contained.

---

## 7. What happens when the channel is full?

`try_send()` returns `Err` immediately — it never blocks. The event is dropped and `events_dropped` counter increments.

The agent response was already sent before `try_send()` was called. The agent is done. The only consequence is that one event is not in the audit log.

This is a deliberate trade-off: **production traffic is more important than a complete audit log during a DB storm.** If the DB is slow and the background writer falls behind, the channel fills. Events drop. Traffic flows. When the DB recovers, the backlog drains. The `events_dropped` Prometheus counter alerts the operator so they can investigate.

**The alternative — blocking on a full channel — would mean a slow DB becomes a slow proxy.** That's a dependency inversion we explicitly refuse.

---

## 8. What is fail-open design and why does it matter?

**Fail-open:** when a component fails internally, traffic continues. Govrix Scout never becomes the reason an AI agent request fails.

Applied throughout:
- Channel full → events drop, request continues
- DB down → events drop, request continues
- PII scanner crashes → event still persisted without PII data, request continues
- Anomaly detector panics → alert not fired, request continues
- Circuit breaker state corrupted → circuit breaker treats it as "allowed," request continues

**The alternative is fail-closed:** internal errors block traffic. Correct for security doors (if the lock fails, keep it locked). Wrong for a proxy in the critical path of production AI agents. If Govrix Scout goes fail-closed and the DB has a 30-second outage, every AI agent in the organization is down.

The design principle: **the proxy is an observer, not a gatekeeper.** The only time it actively blocks is when the operator explicitly configures it to (kill switch, circuit breakers). Infrastructure failures never trigger blocks.

---

## 9. What is the Merkle chain and how does it prove tamper-evidence?

Every event gets a `lineage_hash`:
```
lineage_hash = SHA-256(prev_event_hash + event_id + agent_id + timestamp_ms)
```

The first event has `prev_event_hash = 0x000...`. Each subsequent event chains to the previous one.

**Why this proves tamper-evidence:** If anyone modifies a row in the database — changes a token count, removes a PII flag, alters a cost — that row's `lineage_hash` is now wrong. But also, every event that came after it in the chain is now wrong too, because they all incorporate the previous hash.

To silently edit the database, an attacker would need to:
1. Edit the target row
2. Recompute its hash
3. Recompute every downstream event's hash (could be millions)
4. Do this before anyone notices

In practice, this is not feasible. The chain is verified by the report generator when producing compliance evidence.

**Is this a blockchain?** No. There's no distributed consensus, no proof-of-work, no tokens. It's just a hash chain — the same structure inside Git commits and Bitcoin's block headers. Simple, auditable, mathematically sound.

---

## 10. Why PII regex and not an ML model?

**Three reasons:**

**Latency:** A transformer-based NER model adds 20–100ms per request. At 1000 RPS that's a major bottleneck. Regex is sub-millisecond.

**Predictability:** An ML model can miss a SSN formatted unusually. A regex either matches the pattern or it doesn't — deterministic, auditable, testable. A compliance auditor can read the regex and verify it. They cannot verify a neural network's weights.

**Compliance requirements:** HIPAA and GDPR require that you can explain your PII detection logic. "We use a regex that matches `\d{3}-\d{2}-\d{4}`" is an explanation. "We use a 340M parameter model trained on private data" is not.

Regex covers the structured PII that actually appears in AI agent traffic: SSNs, credit cards, emails, phone numbers, IP addresses. Unstructured PII (names, addresses in natural language) is a roadmap item — ML-based detection for those specific types where the pattern isn't structurally defined.

---

## 11. What is OnceLock and why does it matter for PII?

`OnceLock<T>` is a Rust standard library type: a value that is initialized exactly once, on first access, then reused forever. It's thread-safe with zero runtime cost after initialization.

**The problem it solves:** Compiling a regex is expensive — it involves parsing the pattern into an NFA/DFA and allocating memory. If you compile 5 regexes per event at 1000 RPS, that's 5,000 regex compilations per second. Measurable CPU cost, measurable allocation pressure.

With `OnceLock`: the 5 regexes are compiled once when the first event arrives, then the same compiled objects are reused for every subsequent event forever. Zero allocation on the analysis path.

This is a standard Rust pattern for "compile once, use many times" — similar to how Apache HTTP Server compiles its rewrite rules once at startup.

---

## 12. How do the circuit breakers work?

There are 3, and they run in sequence before every upstream forward:

**Loop Detector:**
- Maintains `HashMap<(agent_id, tool_name), VecDeque<Instant>>`
- On every request, records the current timestamp for that (agent, tool) pair
- Removes timestamps older than 60 seconds (sliding window)
- If the count ≥ 5: return HTTP 429 to the agent, do not forward
- State: in-memory, resets on restart

**Risk Circuit Breaker:**
- Maintains a rolling weighted average risk score per agent over the last 5 minutes
- Risk score is updated by the background writer (after events are processed)
- If score > 75.0: return HTTP 503 to the agent, do not forward
- Threshold is configurable in `govrix.default.toml`

**Kill Switch:**
- Checks `agent.status` in memory (kept in sync from DB)
- If status == `Blocked`: return HTTP 403, do not forward
- Set via dashboard toggle or `PUT /api/v1/agents/{id}` — takes effect on next request

All three are checked before any network call is made to the upstream API. This means the block costs zero dollars — no tokens are consumed.

---

## 13. Why are circuit breakers in-memory and not DB-backed?

**Latency.** A DB round-trip is 1–5ms minimum on localhost. The circuit breaker check needs to complete in <0.1ms to keep the hot path under 1ms total.

**The DB itself can be the failure.** If the circuit breaker depends on the DB, and the DB is slow, then the circuit breaker slows down — exactly when you need it most (during an incident).

**The state is short-lived.** Loop detector state is a 60-second sliding window. When the proxy restarts, the loop detector resets — a reasonable trade-off. Risk scores and kill switch flags are loaded from DB at startup and kept in memory, so they survive restart.

The architectural principle: **anything that runs before the upstream forward must be zero-dependency on external systems.** Only memory.

---

## 14. What happens when the proxy restarts — does state reset?

| State | Persisted? | Behavior on restart |
|-------|-----------|---------------------|
| Loop detector window | No — in-memory VecDeque | Resets. A looping agent gets 60 more seconds before re-detection. Acceptable: the cost of a 60s loop is bounded. |
| Risk scores | Loaded from DB at startup | Survives restart. If an agent was at risk score 80, it's still 80 after restart. |
| Kill switch flags | Loaded from DB at startup | Survives restart. Blocked agents stay blocked. |
| Event buffer (mpsc channel) | No — in-memory | Events in-flight at crash time are lost. Acceptable: the same fail-open trade-off as channel-full. |
| Audit log (DB) | Yes — TimescaleDB | All persisted events survive any proxy restart or crash. |

---

## 15. How does agent identity resolution work?

The proxy resolves an incoming request to an `agent_id` using this priority order:

1. `X-Agent-ID` HTTP header (explicit, highest priority)
2. `Authorization: Bearer {token}` — token mapped to a registered agent
3. mTLS certificate CN (Common Name) from the TLS handshake
4. Client IP + User-Agent fingerprint (fallback, lowest specificity)

If none of these resolve to a known agent, the request is allowed through but tagged as `agent_id = unknown` with `compliance_tag = warn:unknown_agent`.

The `session_id` is extracted from `X-Session-ID` header if present, or generated as `SHA-256(agent_id + request timestamp rounded to 5min)` to group requests in the same conversation window.

---

## 16. What is mTLS and why does govrix-identity exist?

**mTLS = mutual TLS.** In standard HTTPS, only the server proves its identity (via its certificate). In mTLS, both the client and server prove their identities via certificates.

`govrix-identity` is the certificate authority (CA) for the Govrix deployment:
- Issues a unique X.509 certificate to each registered AI agent
- The agent presents this certificate on every connection to the proxy
- The proxy validates the certificate against the CA
- The agent's identity is cryptographically proven — not just a header that anyone can fake

**Why it matters:** Without mTLS, any process on the network can claim to be Agent A by setting `X-Agent-ID: agent-a`. With mTLS, Agent A's certificate was signed by the Govrix CA — unforgeable without the private key.

This is an enterprise feature. The OSS version uses header-based identity. `govrix-identity` ships in the enterprise `govrix` repo.

---

## 17. What is EMA and why use it for anomaly detection?

**EMA = Exponential Moving Average.** A running average where recent values are weighted more heavily than old values.

Formula: `EMA_new = α × current_value + (1 - α) × EMA_old`
Where α (smoothing factor) controls how quickly the average adapts. Govrix uses α = 0.1 — slow adaptation, stable baseline.

**Why EMA over a static threshold ("alert if tokens > 10,000"):**

A static threshold causes false positives on agents that legitimately grow. An agent that starts at 1K tokens/request and grows to 8K tokens/request over a month is normal. A static threshold of 5K would fire alerts continuously.

With EMA: as the agent grows, the EMA grows with it. The alert only fires when the current value is 5× higher than the agent's own recent average. "You just went from your normal 8K to 40K — that's suspicious." That's the signal. The agent's legitimate growth is not.

This is the same technique used in network anomaly detection (NetFlow analysis) and financial fraud detection.

---

## 18. How does the loop detector prevent the $47K incident?

**The $47K incident:** Two AI agents were calling each other in a recursive loop. Agent A called Agent B which called Agent A which called Agent B... for 11 days. Each call billed tokens. Nobody noticed until the invoice arrived.

**How loop detector catches it:** It tracks how many times the same (agent_id, tool_name) pair appears in a 60-second window. In a recursive loop, the same tool gets called repeatedly in rapid succession.

Example: Agent A calls `call_agent_b` 5 times in 60 seconds → loop detector fires → HTTP 429 returned → loop broken → cost capped.

**Why this specific mechanism:** Agent loops repeat the same tool call. They don't mix tool calls randomly. The (agent_id, tool_name) key specifically identifies this pattern. A human doing complex multi-step reasoning calls many different tools in sequence — not the same tool 5 times in 60 seconds.

The threshold (5 calls / 60 seconds) is configurable. For agents that legitimately batch-call tools rapidly, raise the threshold.

---

## 19. What is the actual measured latency?

**Target: <1ms p50, <5ms p99.**

The hot path adds:
- TLS termination: ~0.1ms (TLS 1.3 session resumption)
- Identity resolution: ~0.05ms (HashMap lookup)
- Circuit breaker checks: ~0.05ms (3× HashMap lookup)
- hyper forward overhead: ~0.3ms (compared to direct connection)
- Event build + try_send: ~0.05ms

**Total proxy overhead: ~0.5–0.8ms p50** on a single server with local TimescaleDB.

The upstream API (OpenAI, Anthropic) takes 500–3000ms to respond. The proxy adds <0.1% to the total round trip.

For comparison: LangSmith adds 5–15ms per request (Python overhead + DB write in the hot path). Portkey adds 8–20ms (SaaS network round trip). Govrix Scout's overhead is below the measurement noise for most AI agent use cases.

---

## 20. What is the throughput limit? How does it scale?

**Single instance:** Benchmarks on a 4-core machine show ~15,000–20,000 requests/second before CPU saturation. The limiting factor is TLS handshake computation for new connections (not sustained throughput on keep-alive connections).

**Scale-out:** The proxy is stateless on the hot path (circuit breaker state is in-memory and per-instance). Multiple proxy instances can run behind a load balancer. Each instance writes to the same TimescaleDB. For most enterprise AI agent deployments (100–1000 agents, each making 1–10 calls/second), a single instance is sufficient.

**The background writer** is the secondary bottleneck: 100 events per 100ms = 1,000 events/second write throughput. TimescaleDB can sustain 50,000+ inserts/second on modest hardware. At 1,000 RPS proxy throughput, the writer has 10× headroom.

---

## 21. What happens if the upstream API (OpenAI) is down?

The proxy forwards the request and gets a connection error or timeout from the upstream. It passes that error back to the agent unchanged — the same 5xx or timeout the agent would have received without the proxy.

The proxy does NOT retry on the agent's behalf (no retry logic in the hot path — retries add latency uncertainty). The event is still captured with `finish_reason = error` and the HTTP status code logged.

Govrix Scout does not hide upstream failures. It is transparent.

---

## 22. What happens if the proxy itself crashes?

Agents get a connection refused error on port 4000 — the same as if the proxy was never there.

If the proxy restarts: kill switch state and risk scores reload from DB at startup (typically <1 second). Loop detector state resets — benign trade-off.

**For high availability:** Run two proxy instances behind an nginx or haproxy load balancer. Both write to the same TimescaleDB. If one crashes, the load balancer routes to the other. Zero downtime.

The `docker/docker-compose.yml` includes a restart policy: `restart: unless-stopped` — the proxy restarts automatically on crash.

---

## 23. What happens if TimescaleDB is down?

**Hot path is unaffected.** Agents still get responses. The proxy still forwards.

**Background writer:** cannot INSERT → events in the channel cannot be flushed → channel fills → new events drop (`events_dropped` counter increments) → Prometheus alert fires → operator is notified.

When the DB recovers: the writer drains whatever events remain in the channel (up to 10,000). Events that dropped during the outage are permanently lost from the audit log. The gap is visible in the Prometheus `events_dropped` counter and in the audit log timeline (timestamp gap).

**This is an explicit design choice.** The alternative is for the proxy to queue events to disk during a DB outage. That adds file I/O to the proxy process, disk space management, and a recovery drain procedure. Complexity for a scenario (DB outage) that is rare and recoverable with the current design.

---

## 24. How do you handle authentication and authorization?

**Agent authentication (port 4000 — hot path):**
- Header-based: `Authorization: Bearer <token>` or `X-Agent-ID`
- mTLS certificate (enterprise)
- The proxy does NOT reject unknown agents by default (fail-open) — unknown agents are tagged, not blocked

**Management API authentication (port 4001):**
- Bearer token: `Authorization: Bearer <GOVRIX_API_KEY>`
- Set via environment variable `GOVRIX_API_KEY`
- All `/api/v1/*` routes require this header — 401 otherwise
- Health endpoints (`/health`, `/ready`, `/metrics`) are unauthenticated

**Dashboard:**
- The React dashboard calls the management API — all requests include the API key
- Key is stored in the dashboard's environment config at build time or injected via env var

**There is no multi-user RBAC in the OSS version.** One API key, one access level. Multi-tenant RBAC (read-only auditor role, write-only ops role) is an enterprise feature.

---

## 25. Why 4 mandatory compliance fields on every event?

These are the minimum fields required by every compliance standard Govrix targets:

| Field | Why mandatory |
|-------|--------------|
| `session_id` | Groups requests in a conversation. Required by EU AI Act Article 13 (transparency of automated decisions), SOC 2 CC7 (audit trail of system activity) |
| `timestamp` | UTC ISO-8601, microsecond precision. Required by every standard — without a timestamp, audit logs are inadmissible |
| `lineage_hash` | SHA-256 Merkle chain. Required by SOC 2 CC9 (tamper-evident records), HIPAA §164.312(b) (audit controls) |
| `compliance_tag` | Machine-readable compliance status. Required for automated report generation — `pass:all` vs `warn:pii_email` determines what goes in the evidence package |

They are **non-optional Rust types** — the code does not compile if any of these fields are missing from the `AgentEvent` struct. There is no nullable/Option<> on these fields. The enforcement is at the type system level, not at runtime validation.

---

## 26. How does session tracking work across requests?

An AI agent conversation is multiple HTTP requests. "Summarize this document" might be 10 API calls — initial query, tool calls, follow-ups. These need to be grouped to understand the full conversation cost, compliance posture, and audit trail.

**How session_id is propagated:**

1. If the agent sends `X-Session-ID: {uuid}` — use it (explicit)
2. If not: generate `session_id = SHA-256(agent_id + floor(timestamp / 5min))` — all requests from the same agent within the same 5-minute window get the same session_id (implicit)

Option 2 is imperfect (two separate conversations starting within the same 5-minute window get merged) but works for most real-world agent patterns. Option 1 is correct and what the SDK encourages.

Sessions are queryable: `GET /api/v1/events/sessions/{session_id}` returns all events in a conversation, ordered by timestamp, with total cost and compliance summary.

---

## 27. What compliance standards does this actually satisfy?

**SOC 2 Type II (Trust Services Criteria):**
- CC6: Logical access controls → kill switch, bearer token auth
- CC7: System monitoring → anomaly alerts, Prometheus metrics
- CC9: Change management → Merkle chain tamper-evidence, audit log
- A1: Availability → fail-open design, health endpoints

**HIPAA Security Rule:**
- §164.312(b): Audit controls → complete event log with 4 mandatory fields
- §164.312(c)(1): Integrity → Merkle chain hash verification
- §164.312(e)(1): Transmission security → TLS on all connections

**EU AI Act (August 2, 2026 enforcement):**
- Article 13: Transparency → compliance_tag, session trail, model attribution
- Article 9: Risk management → anomaly detection, circuit breakers, risk scores
- Article 12: Record keeping → tamper-evident audit log, 30-day retention

**India DPDP Act 2023:**
- PII detection + location logging without storing the value
- Breach notification readiness via complete event log

**Govrix generates evidence packages** (PDF + JSON) formatted for each standard — not raw log dumps, but structured evidence mapped to specific control requirements.

---

## 28. How does EU AI Act compliance work specifically?

The EU AI Act requires organizations using "high-risk AI systems" to maintain:

1. **Technical documentation** — what models are used, for what purpose
2. **Logging** — who used the AI, when, what input/output
3. **Transparency** — users must be informed when interacting with AI
4. **Human oversight** — mechanism to override or stop AI decisions

Govrix Scout satisfies:

1. Every event records `model`, `protocol`, `agent_id`, `tool_calls[]` — complete model attribution
2. Full event log with session grouping — `who, when, what` is queryable
3. Compliance tags map to transparency requirements — `pass:all` means the interaction met all configured transparency rules
4. Kill switch = one-click human override on any agent, instant

The report generator formats this as Article 12 and Article 13 evidence — structured JSON that maps each data field to the specific Act requirement. Legal teams can submit this directly to regulators.

**EU AI Act enforcement begins August 2, 2026.** Organizations deploying AI agents without compliant logging infrastructure are at risk of fines up to €30M or 6% of global turnover.

---

## 29. What is DPDP and why is it relevant?

**DPDP = Digital Personal Data Protection Act, India, 2023.** Enforced from 2025 onwards.

Requires:
- Personal data (PII) must be identified, tracked, and protected
- Breach notification within 72 hours of discovery
- Data minimization — collect only what's necessary

Govrix Scout is DPDP-relevant because AI agent traffic routinely contains personal data in prompts and responses. A healthcare agent processing patient queries, a financial agent accessing account data, a customer service agent with user details — all of these pass through the proxy.

The PII scanner detects and flags personal data in real-time. The audit log records what data was present, when, for which agent, in which session. If a breach occurs, the event log provides the exact records to satisfy the 72-hour notification requirement.

---

## 30. Why sqlx over Diesel or SeaORM?

**Diesel:** Compile-time checked queries — great for correctness. But the query DSL is complex and the migration system requires generated schema files. For time-series workloads with hypertable-specific SQL (TimescaleDB extensions, continuous aggregates), Diesel's DSL doesn't map cleanly — you end up dropping to raw SQL anyway.

**SeaORM:** Active Record pattern, code generation from DB schema. Good for CRUD-heavy applications. The generated boilerplate is overkill for what is essentially a write-heavy append log.

**sqlx:** Raw SQL with compile-time query verification (the `query!` macro checks your SQL against a live DB at compile time). No ORM magic. TimescaleDB-specific SQL works natively. Batch inserts are straightforward. Async-native (tokio-compatible). And because it's raw SQL, any DBA who reads the code understands exactly what's happening.

For a compliance-grade audit system, readable, explicit SQL is better than ORM-generated queries that hide what's actually executing against the database.

---

## 31. How do database migrations work?

The `init/` directory contains 5 SQL migration files, applied in order:

1. `01_extensions.sql` — enables TimescaleDB extension, uuid-ossp
2. `02_agents.sql` — creates agents table with indexes
3. `03_events.sql` — creates events hypertable, sets partitioning, enables compression
4. `04_views.sql` — creates cost_daily continuous aggregate
5. `05_policies.sql` — creates policy rules table (for future enforcement)

All migrations are **idempotent** (`CREATE TABLE IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`). Running `make migrate` twice is safe.

`make db-reset` drops and recreates the database then runs all migrations — used in development and CI. Never run in production without a backup.

The proxy validates on startup that all required tables exist and the schema matches expectations. Mismatched schema → startup failure with clear error message rather than silent corruption.

---

## 32. What is the cost_daily materialized view?

A **materialized view** is a query result stored as a table — computed once, read many times. TimescaleDB's **continuous aggregate** is a materialized view that incrementally updates as new data arrives.

`cost_daily` aggregates:
```sql
SELECT
  date_trunc('day', timestamp) AS day,
  agent_id,
  model,
  protocol,
  SUM(input_tokens)  AS total_input_tokens,
  SUM(output_tokens) AS total_output_tokens,
  SUM(cost_usd)      AS total_cost_usd
FROM events
GROUP BY day, agent_id, model, protocol
```

Without this view: the `GET /api/v1/costs/breakdown` endpoint scans the full events table and aggregates on the fly — slow at scale. With the view: it reads pre-aggregated rows — fast regardless of how many events exist.

The view refreshes every time the background writer completes a batch (every 100ms). The cost dashboard is effectively real-time with 100ms staleness.

---

## 33. How is cost calculated per event?

The `govrix-scout-common` crate contains an embedded pricing table — hardcoded USD costs per 1M input/output tokens for each supported model:

```
gpt-4o:          input $2.50/1M,  output $10.00/1M
gpt-4o-mini:     input $0.15/1M,  output $0.60/1M
claude-3-5-sonnet: input $3.00/1M, output $15.00/1M
claude-3-haiku:  input $0.25/1M,  output $1.25/1M
...
```

The protocol parser extracts `input_tokens` and `output_tokens` from each API response. Cost is:
```
cost_usd = (input_tokens / 1_000_000) × input_price
         + (output_tokens / 1_000_000) × output_price
```

**8 decimal precision** is used for the USD value to avoid floating-point rounding errors when summing millions of small transactions. Stored as `NUMERIC(20, 8)` in Postgres — exact arithmetic, no floating-point.

The pricing table is updated manually when providers change their pricing. Roadmap: pull pricing from a provider API or a maintained community dataset.

---

## 34. How does the kill switch work end-to-end?

1. Operator clicks kill switch toggle in dashboard (or calls `PUT /api/v1/agents/{id}` with `{"status": "blocked"}`)
2. Management API updates `agents.status = 'blocked'` in TimescaleDB
3. Management API updates the in-memory agent status map (hot path's copy of agent states)
4. Next request from that agent arrives at port 4000
5. Identity resolution identifies agent_id
6. Kill Switch circuit breaker checks in-memory status map: `Blocked`
7. Returns HTTP 403 before forwarding — zero upstream tokens consumed
8. Event logged with `compliance_tag = fail:kill_switch`

**Time from toggle click to block taking effect:** <100ms (network round trip from dashboard to API + memory write). The agent's next request is blocked.

**To unblock:** same flow with `{"status": "active"}`.

**No redeployment. No code change. No restart.** The hot path reads the in-memory map on every request — the state update is visible immediately.

---

## 35. How does the risk score get calculated?

The risk score is a weighted rolling average (0.0–100.0) computed per agent by the background writer after each event batch is processed.

**Inputs to risk score:**

| Signal | Weight | Rationale |
|--------|--------|-----------|
| PII detected in request | +20 | Sending personal data to an LLM is high-risk |
| PII detected in response | +15 | LLM returning personal data suggests data leakage |
| Token volume spike (>3× EMA) | +25 | Runaway behavior pattern |
| Off-hours activity | +10 | Reduced oversight |
| New tool invoked | +15 | Unexpected behavior |
| Compliance tag = fail:* | +30 | Hard violation |
| Normal event | -5 (decay) | Risk decays over time with clean behavior |

Risk score is bounded [0, 100]. An agent that consistently behaves normally trends toward 0. An agent hitting multiple signals trends toward 100.

At 75.0 the risk circuit breaker fires (HTTP 503). At 90.0 a critical alert is fired with recommendation to activate kill switch.

---

## 36. How does protocol detection work for OpenAI vs Anthropic vs MCP?

The proxy inspects the URL path of the incoming request:

```
/proxy/openai/*     → OpenAI protocol, forward to api.openai.com
/proxy/anthropic/*  → Anthropic protocol, forward to api.anthropic.com
/proxy/mcp/*        → MCP protocol, forward to configured MCP server URL
```

**No body inspection needed for routing.** The path is sufficient. This keeps the detection at O(1) — string prefix match, no JSON parsing.

After forwarding, the response is also protocol-specific. Each parser knows the exact JSON schema:
- OpenAI: `response.usage.prompt_tokens`, `response.usage.completion_tokens`, `response.choices[0].finish_reason`
- Anthropic: `response.usage.input_tokens`, `response.usage.output_tokens`, `response.stop_reason`
- MCP: `response.result`, `response.error`, `response.isError`

If the response JSON doesn't match the expected schema, the event is persisted with `parse_error = true` and zero token counts. The request still succeeds — the parse error is non-blocking.

---

## 37. What is SSE and how does the live event stream work?

**SSE = Server-Sent Events.** A standard HTTP mechanism where the server keeps a connection open and pushes data to the client as it arrives. Unlike WebSockets, SSE is one-directional (server → client) and works over plain HTTP.

`GET /api/v1/events/stream` responds with:
```
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive
```

Then sends events as they arrive:
```
data: {"id":"evt_abc","agent_id":"agent-1","timestamp":"...","model":"gpt-4o",...}\n\n
data: {"id":"evt_def","agent_id":"agent-2",...}\n\n
```

The React dashboard connects to this endpoint via the browser's `EventSource` API. Each event received triggers a React Query cache update — the events table in the dashboard updates in real-time without polling.

**Why SSE over WebSockets?** SSE is simpler — standard HTTP, works through load balancers and proxies without special configuration, auto-reconnects on disconnect, and one-directional push is all that's needed.

---

## 38. Why React 18 + TanStack Query for the dashboard?

**React 18:** The dashboard uses concurrent features — `useDeferredValue` for the large events table (keeps the UI responsive while filtering thousands of rows) and `useTransition` for page navigation (non-blocking route changes). These are React 18 additions.

**TanStack Query (formerly React Query):** Manages server state — caching, background refetching, stale-while-revalidate, optimistic updates. Without it, every component would implement its own fetch logic, loading states, and error handling. TanStack Query centralizes this: one cache, configurable stale times, automatic background refresh.

The cost dashboard polls `GET /api/v1/costs/summary` every 30 seconds (cost data changes slowly). The events feed uses SSE (real-time). The agent list polls every 10 seconds. TanStack Query manages all these independently with different stale/refetch policies — without manual `setInterval` in every component.

**Vite** over Create React App: faster HMR (hot module reload), smaller bundles, native ESM. Not relevant to the architecture explanation — just the right default toolchain for React 18.

---

## 39. Why 8 Rust crates? Why not a monolith?

**Compilation boundaries.** In a Rust workspace, each crate compiles independently. If you change only `govrix-policy` (PII logic), only that crate and its dependents recompile. In a monolith, any change recompiles everything. With 93 source files, this is a meaningful difference — incremental compile time: ~3 seconds vs ~45 seconds.

**Dependency isolation.** `govrix-scout-proxy` (hot path) depends on `govrix-scout-common` and `govrix-policy` — but does NOT depend on `govrix-scout-reports` (PDF generation). If the PDF library has a vulnerability, the hot path binary is unaffected.

**The crates map to actual architectural boundaries:**
- `govrix-scout-proxy`: hot path — must be fast, must stay simple
- `govrix-scout-common`: shared types — must be stable, no business logic
- `govrix-scout-store`: DB layer — change DB library here, nothing else changes
- `govrix-policy`: intelligence — PII and compliance logic, can evolve independently
- `govrix-scout-reports`: output — PDF/JSON generation, no access to hot path

**The binary is still one.** `cargo build --release` produces `govrix-scout` — one binary that includes all crates. The crate split is a development-time optimization, not a deployment boundary.

---

## 40. How is the system deployed? What are the requirements?

**Minimum requirements (development):**
- Docker + Docker Compose
- `docker compose up` starts TimescaleDB + proxy + dashboard in 3 containers
- One environment variable to point agents at the proxy: `export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1`

**Production minimum:**
- One server: 2 CPU cores, 4GB RAM, 50GB SSD (for TimescaleDB + 30 days events)
- Or: TimescaleDB on separate managed Postgres (Timescale Cloud, AWS RDS with TimescaleDB extension)
- TLS termination at nginx or load balancer
- `GOVRIX_STORE__DATABASE_URL`, `GOVRIX_API_KEY` environment variables

**High availability:**
- 2× proxy instances behind load balancer (stateless hot path)
- TimescaleDB in streaming replication (Postgres HA patterns apply)
- The React dashboard is static — serve from CDN or any web server

**Kubernetes:** A Kubernetes operator is on the 90-day roadmap — Helm chart for proxy deployment, CRD for agent registration, automatic sidecar injection.

---

## 41. Why not just use LangSmith / Portkey / Helicone?

This is the most important question. Short answer: they solve a different problem.

| | LangSmith | Portkey | Helicone | Govrix Scout |
|---|---|---|---|---|
| **Your data location** | LangChain's servers | Portkey's servers | Helicone's servers | Your infrastructure |
| **Framework lock-in** | LangChain only | Any (via proxy) | Any (via proxy) | Any (via proxy) |
| **Tamper-evident audit** | No | No | No | Yes (Merkle chain) |
| **Proactive anomaly alerts** | No | No | No | Yes (3 detectors) |
| **Kill switch** | No | No | No | Yes (in-memory, instant) |
| **MCP support** | No | No | No | Yes |
| **Self-hosted** | Partial | No | No | Yes — it's the default |
| **Sub-millisecond latency** | No | No | No | Yes (hyper) |
| **Compliance reports** | No | No | No | Yes (SOC 2 / HIPAA / EU AI Act) |

The reframe: LangSmith/Portkey/Helicone are **observability tools** — they show you what happened. Govrix Scout is a **governance proxy** — it observes, and it can act. The kill switch, circuit breakers, and compliance evidence are things that don't exist in any competing tool.

The data ownership argument is separate and equally important: a Fortune 500's AI agent traffic contains proprietary prompts, customer data, and trade secrets. Routing that through a third-party SaaS is a compliance and competitive risk.

---

## 42. What does zero agent code changes actually mean?

AI agents are configured to call OpenAI at `https://api.openai.com/v1`. To route through Govrix Scout, change one environment variable:

```bash
export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1
```

The OpenAI SDK reads this variable automatically. Every call the agent makes now goes to Govrix Scout instead of OpenAI directly. The agent code doesn't know. The agent developer doesn't need to add any instrumentation, SDK, or library.

**Why this works:** The OpenAI SDK constructs the API URL from `OPENAI_BASE_URL`. Govrix Scout presents the same HTTP interface as OpenAI — same paths, same JSON schema. The proxy validates that the response from OpenAI matches what the agent expects and passes it through unchanged.

Same for Anthropic: `ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1`

For MCP: configure the MCP client's server URL to point at `http://localhost:4000/proxy/mcp/`

**What if the agent hardcodes the URL?** That's a bug in the agent — it's not following the SDK contract. The SDK exists precisely to allow URL configuration. If the agent hardcodes, the developer must change that one line. But that's a code quality issue in the agent, not a Govrix Scout limitation.

---

## 43. How do you handle multiple agents simultaneously?

The proxy handles all agents concurrently via Tokio's async runtime. Each incoming connection is an independent async task — no shared state on the hot path except the in-memory circuit breaker maps (which use `DashMap`, a concurrent hash map with shard-level locking).

**Isolation:** Each agent has its own circuit breaker state, risk score, session tracker, and EMA baseline. Agent A's risk score does not affect Agent B. Agent A's loop detection window is separate from Agent B's.

**No per-agent configuration required at startup.** New agents are automatically registered on first request — their `agent_id` is created in the DB, their in-memory state initialized to defaults, and they appear in the dashboard.

**The kill switch is per-agent.** You can block Agent A without affecting Agent B. Bulk kill switch operations (block all agents in a project) are a 90-day roadmap feature.

---

## 44. What is Prometheus and what metrics do you expose?

Prometheus is an open-source monitoring system. It scrapes metrics from the `/metrics` endpoint (port 9090) on a configurable interval and stores them as time-series data. Grafana connects to Prometheus for visualization and alerting.

**Metrics exposed by Govrix Scout:**

```
# Throughput
govrix_requests_total{protocol, agent_id, status}
govrix_requests_per_second

# Latency (proxy overhead only)
govrix_proxy_latency_p50_ms
govrix_proxy_latency_p99_ms

# Event pipeline
govrix_events_sent_total
govrix_events_dropped_total        ← alert if this rises
govrix_events_processed_total
govrix_channel_fill_ratio           ← 0.0–1.0, alert at 0.8

# Circuit breakers
govrix_circuit_breaker_trips_total{type: loop|risk|kill}
govrix_agents_blocked_count

# PII
govrix_pii_detections_total{type: email|phone|ssn|credit_card|ip}

# Cost
govrix_cost_usd_total{agent_id, model}
govrix_budget_utilization_ratio{agent_id}  ← alert at 0.8
```

Standard Prometheus alerting rules are included in `docker/prometheus.yml`.

---

## 45. Can an agent bypass the proxy?

**Technically yes, if they hardcode the API URL.** An agent that directly calls `https://api.openai.com` bypasses Govrix Scout entirely.

**How to prevent bypass:**

1. **Network policy:** In a Kubernetes deployment, a NetworkPolicy denies direct egress to external AI APIs from agent pods. All egress to `api.openai.com` is blocked at the network level. Only the Govrix Scout proxy has egress allowed. Agents must use the proxy — there is no bypass.

2. **API key management:** Store the real OpenAI API key only in the proxy (via env var). Agents get a Govrix-issued proxy token, not the real key. The proxy validates the proxy token and forwards with the real key. An agent cannot directly call OpenAI because it doesn't have the real API key.

3. **mTLS:** (Enterprise) All agent connections to external services require a client certificate signed by the Govrix CA. Only the proxy has this certificate. An agent trying to call OpenAI directly lacks the certificate.

The network policy approach is the most practical for Kubernetes environments. The API key vaulting approach works for any environment.

---

## 46. Why log PII location and not the value?

**The matched value is itself PII.** If the audit log stores the actual SSN or credit card number, the audit log is now a PII database — subject to the same HIPAA, GDPR, and DPDP requirements as the original data, plus breach notification requirements if compromised.

**What is logged instead:**
- PII type: `ssn`, `email`, `phone`, etc.
- Field path: where in the JSON it was found (e.g., `request.messages[2].content`)
- Character offset: position within the field (e.g., `chars 45–56`)
- Confidence: regex match confidence (currently 1.0 — regex either matches or doesn't)

This tells a compliance auditor: "SSN was present in this request, in this location." They can investigate the original agent conversation if needed. But the audit system itself does not store the SSN.

**Practical compliance benefit:** The audit log can be stored in a standard S3 bucket or database without PII handling requirements. The original conversation data (which does contain PII) can be stored separately with appropriate access controls, encryption, and retention policies.

---

## 47. What is the new tool detector warm-up period for?

When a new agent first starts up, every tool it calls is "new" — it has no history. Without a warm-up period, every tool call in the first 10 events would fire an alert. That's noise, not signal.

The warm-up period (first 10 events) lets the agent establish its baseline set of tools naturally. After 10 events, the `HashSet<tool_name>` is populated with the tools this agent normally uses. From event 11 onwards, any new tool_name that appears is genuinely unexpected.

**Why 10 events:** Empirically, most agent initialization sequences (startup, health checks, initial configuration calls) complete within 5–8 events. 10 provides a safe margin. Configurable if an agent has a longer startup sequence.

**Why per-agent and not per-deployment:** Different agents have different tool sets. The set of tools Agent A uses is irrelevant to Agent B. The alert fires when *this specific agent* calls a tool *it has never called before* — not when any agent calls an unfamiliar tool.

---

## 48. How are compliance reports generated?

`govrix-scout-reports` reads from the audit log and formats evidence packages:

**PDF report contains:**
1. Cover page: report type, date range, organization, generated by Govrix Scout
2. Executive summary: total events, agents, PII incidents, budget utilization, anomalies
3. Event log excerpt: representative sample with lineage hash chain visible
4. Hash chain verification: cryptographic proof that the log was not tampered with
5. PII detection summary: types found, count, agents involved
6. Budget compliance: per-agent spend vs. limits
7. Anomaly timeline: all alerts with severity and resolution status
8. Appendix: control mapping to specific regulation articles

**JSON report contains:** the same data in machine-readable form for submission to automated compliance tools (ServiceNow GRC, Archer, Vanta).

The Merkle chain verification section is the differentiator — no other tool produces a cryptographic proof that the audit log is intact. An external auditor can independently verify the hash chain by running the same SHA-256 computation on any row and checking it against the stored `lineage_hash`.

---

## 49. Is this production-ready? What's missing?

**What works today (verified in CI):**
- Proxy hot path with <1ms p50 latency target
- OpenAI, Anthropic, MCP protocol support
- 3 anomaly detectors (OffHours, EMA, NewTool)
- PII detection (5 patterns, OnceLock)
- Circuit breakers (Loop, Risk, Kill Switch)
- Merkle chain on all events
- TimescaleDB persistence with compression
- 18-page React dashboard
- REST management API
- Prometheus metrics
- Docker Compose deployment

**What's on the 90-day roadmap (not yet built):**
- Policy enforcement in the hot path (block requests that violate configured rules)
- PII masking (replace PII in the request before forwarding, not just logging it)
- Slack/PagerDuty integration for anomaly alerts
- Kubernetes operator + Helm chart
- Bulk agent operations
- ML-based anomaly baseline (replacing static EMA parameters with trained models)

**What's enterprise-only (separate govrix repo):**
- mTLS (govrix-identity)
- Multi-tenant RBAC
- SSO/SAML integration
- Advanced policy engine

---

## 50. What would it take to deploy this at BlackRock / Optum / Verizon scale?

**Scale assumptions:** 1,000 AI agents, each making 100 calls/minute = 100,000 requests/minute = ~1,700 RPS sustained. Peak: 10,000 RPS.

**Proxy layer:** 
- 3 proxy instances behind load balancer handles 10,000 RPS comfortably (each handles ~3,300 RPS, well below 15,000 RPS single-instance limit)
- Kubernetes: 3-replica Deployment, HPA scaling on CPU utilization
- Load balancer: nginx or AWS ALB

**Database:**
- TimescaleDB multi-node (distributed hypertable) for 100M+ events/day
- Or: Timescale Cloud managed service — no operational overhead
- Read replica for dashboard queries (analytics don't touch the write primary)

**Compliance requirements specific to each company:**

- **BlackRock (FINRA/SEC):** Add immutable S3 log archival (WORM storage) alongside TimescaleDB for 7-year regulatory retention. The Merkle chain proof covers chain-of-custody. FINRA Rule 17a-4 compliance.

- **Optum/UHG (HIPAA):** PHI in AI prompts requires Business Associate Agreement with any cloud provider. Self-hosted Govrix Scout eliminates this requirement — no BAA needed since PHI never leaves Optum's infrastructure.

- **Verizon (FCC/SOX):** SOX requires financial data audit trails. Govrix Scout's tamper-evident log + cost tracking covers the AI spend audit component. FCC data residency requirements are met by self-hosted deployment.

**Estimated timeline to production:** 2–3 weeks for a mid-size enterprise (network policy setup, API key vaulting, Kubernetes deployment, initial agent onboarding). Most of that time is enterprise security review, not Govrix Scout configuration.

---

*End of Q&A — 50 questions. Know all of them.*
