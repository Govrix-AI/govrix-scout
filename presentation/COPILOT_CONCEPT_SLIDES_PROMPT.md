# Microsoft Copilot — Modification Prompt
### This presentation already has 19 slides. Paste this prompt to add 3 new slides and make targeted edits.

---

## PROMPT — COPY FROM HERE

---

This presentation already has 19 slides built from a previous prompt. Do not recreate or reorder any existing slides. I need you to do two things only:

1. Make targeted edits to 2 existing slides
2. Insert 3 new slides between the current slide 17 (Anomaly Engine) and slide 18 (Accomplishments & Roadmap)

---

## PART 1 — EDITS TO EXISTING SLIDES

**Edit slide 4 (Problem Statement) — Column 3 only**
Change the body text of the third column from whatever references the $47K loop to:
"Change Healthcare: 192.7M patient records exposed. Agentic AI with no runtime compliance layer. The breach exists. The audit trail does not."

**Edit slide 10 (Traction) — remove one line from the right panel**
Remove the line: "docker compose up → 1 env var → governed. Your data stays on your infra."
This is already said on slide 5. Do not replace it with anything — just remove that line.

---

## PART 2 — INSERT 3 NEW SLIDES

Insert the following 3 slides between the current slide 17 (Anomaly Engine) and slide 18 (Accomplishments & Roadmap). Use the "Discover Our Services" card grid layout for all 3 — 2×2 grid, 4 cards per slide. Use the template's existing design exactly as-is.

---

### NEW SLIDE A — "WHY WE BUILT IT THIS WAY"

**Slide title:** "Why We Built It This Way"
**Subtitle:** "Four decisions. Each one made after hitting the wrong wall first."

**Card 1 — Why Rust and not Python?**

Python has the GIL — Global Interpreter Lock. Only one thread executes Python at a time. LangSmith, Portkey, Helicone are all Python. At ~500 RPS they saturate. LiteLLM hits this ceiling in production.

Rust has no GC. No GIL. No runtime pauses. Every connection is an independent async task. Latency is deterministic — no garbage collector running unpredictably mid-request.

The same reason AWS Firecracker, the Linux kernel's eBPF subsystem, and Cloudflare's edge runtime are written in Rust: deterministic performance at the boundary of a system.

**Card 2 — Why hyper and not axum for the proxy?**

axum is built on top of hyper. It adds a routing layer, middleware stack, and handler abstractions — right for a REST API, wrong for a proxy.

Every axum request: router matching → middleware chain → handler extraction → response serialization. That adds ~300µs per request for work the proxy never needs to do.

The proxy does one thing: receive bytes, check 3 in-memory flags, forward bytes unchanged. Raw hyper skips the entire axum stack. The management API on port 4001 uses axum — that is the right place for it.

**Card 3 — Why a bounded channel and not inline DB writes?**

The first version wrote to the database before returning the response. A slow database made a slow proxy. That is a production incident the proxy caused.

The fix: bounded mpsc channel with try_send().

try_send() returns instantly. The channel holds up to 10,000 events in memory. The agent has its response before a single DB write happens. DB slow or down — events drop, traffic flows. The agent never knows the database state.

**Card 4 — Why TimescaleDB and not plain Postgres?**

Plain Postgres was the starting point. At 1 million events per day, a query for "last 24 hours" became a full table scan. 2 to 5 second response times at 30 days of data.

TimescaleDB is a Postgres extension — same sqlx driver, same SQL syntax, same DATABASE_URL. Internally it partitions the table by time. A 24-hour query touches only today's chunk.

Query time: 10× faster. Chunks older than 7 days auto-compress (~90% smaller). Chunks older than 30 days auto-drop.

---

### NEW SLIDE B — "THE CONCEPTS BEHIND THE CODE"

**Slide title:** "The Concepts Behind the Code"
**Subtitle:** "Four ideas borrowed from cryptography, systems design, and signals processing."

**Card 1 — What is the Merkle chain?**

Every event gets a lineage hash:

  lineage_hash = SHA-256(prev_hash + event_id + agent_id + timestamp_ms)

Edit any row — change a token count, remove a PII flag — and that row's hash is wrong. Every event after it is also wrong because they all chain from the previous hash.

To tamper silently: recompute millions of hashes before anyone checks. In practice: not feasible.

This is the same structure inside Git commit history. Every commit hashes its parent. An auditor verifies it by re-running SHA-256 on any row independently.

**Card 2 — What is OnceLock?**

PII detection runs 5 regex patterns per event. Compiling a regex allocates memory and parses the pattern into a state machine. At 1,000 requests/second: 5,000 regex compilations per second. Measurable CPU waste.

OnceLock<T> initializes a value exactly once — on first access — then reuses it forever across all threads at zero cost.

  static PII_PATTERNS: OnceLock<PiiPatterns> = OnceLock::new();

5 patterns compiled once at proxy startup. Every event after that: zero allocation on the analysis path. Apache HTTP Server compiles rewrite rules the same way.

**Card 3 — What is EMA and why not a fixed threshold?**

EMA = Exponential Moving Average:

  EMA_new = 0.1 × current_tokens + 0.9 × EMA_prev

A fixed threshold caused constant false positives on agents that grew from 1K to 8K tokens per request over a month. Every alert was noise — the agent was growing, not misbehaving.

EMA adapts. The baseline rises with the agent. An alert fires only when the current event is 5× the agent's own recent average — not 5× a global number. Growth is not anomaly. Sudden spike is.

Same technique used in Netflix CDN traffic analysis and financial fraud detection.

**Card 4 — Why PII regex and not an ML model?**

A transformer NER model adds 20–100ms per request. At 1,000 RPS that is a bottleneck before the database write.

HIPAA and GDPR require that PII detection logic is explainable to a regulator. "\d{3}-\d{2}-\d{4} matches Social Security Numbers" is an explanation. A 340M parameter model is not.

Regex is deterministic, auditable, and fast. Patterns applied in specificity order — SSN first, IP last — prevent overlapping digit sequences from matching twice. Result logged: type and field offset only. The matched value is never stored.

---

### NEW SLIDE C — "WHY NOT THE OBVIOUS CHOICE?"

**Slide title:** "Why Not the Obvious Choice?"
**Subtitle:** "Every tool below was evaluated. Here is why we did not use it."

**Card 1 — Why not Kafka or Redis Streams for the event queue?**

mpsc = multi-producer, single-consumer. Tokio's mpsc channel is a lock-free in-process queue. The proxy and the background writer are in the same process — communication is memory, not TCP.

Kafka adds: a network hop (~1ms even on localhost), a broker that can crash independently, serialization overhead, and consumer group coordination. Kafka is right when producers and consumers are on different machines. They are not.

Redis Streams has the same problem: a network call in the path of a <1ms hot path. An external process that the proxy now depends on. If Redis is down, the proxy blocks or loses events silently.

**Card 2 — Why not MongoDB or Cassandra for storage?**

MongoDB's design is flexible schema. The requirement here is the opposite — 4 mandatory compliance fields on every event, enforced at the Rust type system level and again as SQL NOT NULL. MongoDB's flexibility means those fields can be absent. That is not a bug in MongoDB. It is the wrong design for a compliance audit log.

Cassandra handles 100M+ events per day across multiple data centres. Govrix Scout targets 1M events per day on a single server. Its query model also makes time-range scans per agent painful — optimised for known partition keys, not ad-hoc filters.

TimescaleDB is Postgres. The DBA already knows it.

**Card 3 — Why not Diesel or SeaORM?**

Diesel generates a compile-time DSL over SQL. For standard CRUD it is excellent. For TimescaleDB-specific SQL — continuous aggregates, hypertable creation, chunk compression policies — the DSL does not map and you escape to raw SQL anyway.

SeaORM uses Active Record with code generation from the schema. The events table is an append-only audit log — Active Record patterns add abstraction that serves no purpose for a write-once-read-many workload.

sqlx compiles raw SQL against a live database at build time using the query! macro. SQL verified correct at compile time. No ORM layer hiding what executes. TimescaleDB-specific syntax works natively. A DBA reading the codebase sees SQL.

**Card 4 — Why not LangSmith, Portkey, or Helicone?**

They are observability tools. They show you what happened. Govrix Scout is a governance proxy — it observes and it acts.

The kill switch does not exist in any of them. The Merkle chain tamper-proof audit does not exist in any of them. MCP support does not exist in any of them.

The deeper issue: AI agent traffic contains proprietary prompts, customer data, and session context. LangSmith routes this through LangChain's servers. Portkey through Portkey's. Helicone through Helicone's.

For a HIPAA-regulated organisation, routing patient-related agent traffic through a third-party SaaS is not a product decision — it is a compliance violation. Self-hosted is not a preference. It is a hard requirement.

---

## PLACEMENT AFTER INSERTION

The presentation should have 22 slides in this order:

Slides 1–17: unchanged (existing slides)
Slide 18: ← NEW "Why We Built It This Way"
Slide 19: ← NEW "The Concepts Behind the Code"
Slide 20: ← NEW "Why Not the Obvious Choice?"
Slide 21: Accomplishments & Roadmap (was slide 18)
Slide 22: Thank You (was slide 19)

---

## INSTRUCTIONS FOR ALL NEW SLIDES

- Use monospace font for all code and formula lines — small size, left-aligned
- Do not use code block decorations or syntax highlighting boxes
- Keep non-code body text tight — remove any word that does not add meaning
- Use the template's accent color on these terms wherever they appear: OnceLock, SHA-256, try_send(), EMA, hyper, mpsc, GIL
- Do not add slide numbers manually — let the template handle it
- Do not change any colors, fonts, or layouts on existing slides

---

*End of prompt*
