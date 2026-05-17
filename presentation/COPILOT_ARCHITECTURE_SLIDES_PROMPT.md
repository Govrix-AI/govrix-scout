# Microsoft Copilot — Add Architecture Slides Prompt
### Paste into Copilot AFTER the existing 14 slides are filled with content
### These slides go AFTER slide 10 (Traction), BEFORE the Accomplishments/Roadmap slide

---

## FIRST — CUTS TO MAKE BEFORE ADDING NEW SLIDES

Before adding new slides, make these edits to existing slides:

**DELETE Slide 13 (Fail-Open / Bounded Channel)**
This concept is already covered in Slide 9 (Advantage 03) and Slide 16 (Circuit Breakers intro). Remove the entire slide. Renumber subsequent slides.

**EDIT Slide 4 (Problem Statement) — Column 3**
Change: "A $47K loop ran for 11 days. Nobody was notified."
To: "Change Healthcare: 192.7M patient records exposed. Agentic AI with no runtime compliance layer."

**EDIT Slide 10 (Traction) — right panel bottom text**
Remove: "docker compose up → 1 env var → governed. Your data stays on your infra."
(This is already on Slide 5 — "One line. Done.")

---

## PROMPT — COPY FROM HERE

---

I need you to **add 8 new slides** to this presentation after the "Traction" slide. These slides show the inner technical architecture of Govrix Scout — how it actually works under the hood, including the real problems we hit and the exact decisions we made to solve them. Use the template's existing slide layouts and design exactly as-is. Do not change any colors or fonts. Just use the closest matching layout for each slide described below.

---

### NEW SLIDE A — SYSTEM ARCHITECTURE DIAGRAM
**Use:** The "Our Innovative Solutions" split layout — left title panel, right content panel

**Left panel title (bold, stacked):**
"System"
"Architecture"

**Left panel small text below:** "One binary. Zero agent code changes. Every AI call captured."

**Right panel:**
Insert the architecture diagram image here (PNG exported from mermaid.live — see separate diagram guide).

If image is not available, use this text diagram instead:

```
[AI Agent]
    ↓  HTTP  port 4000
[govrix-scout-proxy — hyper]
    ↓ in-memory checks
[Circuit Breakers] → BLOCK (429/403/503)
    ↓ allowed
[Upstream API: OpenAI / Anthropic / MCP]
    ↓ response → back to agent immediately
    ↓ try_send() — never blocks
[Bounded Channel  mpsc 10,000]
    ↓ drain 100 events / 100ms
[Background Writer]
    ├── PII Scanner (OnceLock regex)
    ├── Anomaly Detectors (EMA / OffHours / NewTool)
    ├── Cost Calculator (USD / 1M tokens)
    └── Merkle Chain (SHA-256)
    ↓
[TimescaleDB hypertable]
    ↓
[axum REST API  port 4001] → [React Dashboard  port 3000]
```

---

### NEW SLIDE B — THE WORKSPACE ARCHITECTURE
**Use:** The "Discover Our Services" layout — card grid with titled cards

**Slide title:** "How It's Built"
**Subtitle small:** "8 Rust crates. 93 source files. One binary."

**Cards (one per crate):**

Card 1 — **govrix-scout-proxy**
The hot path. Intercepts all AI agent traffic. Uses `hyper` directly — NOT axum — for sub-millisecond overhead. Also serves the REST management API on port 4001 via axum on a separate thread. Binary: `govrix-scout`.

Card 2 — **govrix-scout-common**
Shared data models across all crates. Defines `AgentEvent`, `Agent`, `Cost` schemas. The 4 mandatory compliance fields are enforced here as non-optional Rust types.

Card 3 — **govrix-scout-store**
The persistence layer. PostgreSQL + TimescaleDB via `sqlx`. Handles batch inserts, materialized view refreshes, agent upserts, budget tracking.

Card 4 — **govrix-policy**
The intelligence layer. PII detection (5 regex patterns). Budget evaluation. Compliance tagging. Session tracking. Policy engine. All off the hot path.

Card 5 — **govrix-identity**
mTLS identity layer. Certificate authority, cert issuance, mutual TLS handshake validation. Gives every agent a cryptographic identity.

Card 6 — **govrix-scout-reports**
PDF and JSON compliance report generation. Reads from the audit log. Generates SOC 2, HIPAA, EU AI Act evidence packages.

---

### NEW SLIDE C — THE HOT PATH: WHAT HAPPENS IN <1ms
**Use:** The "Problem Statement" layout — large header, 3 equal columns below with icon and text

**Slide title:** "The Hot Path"
**Small subtitle:** "Zero DB round-trips. Zero blocking I/O. Zero added latency."

**Column 1:**
**STEP 1 — INTERCEPT (0ms)**
`hyper` server receives request on port 4000. Protocol detected from URL: /proxy/openai → OpenAI, /proxy/anthropic → Anthropic, /proxy/mcp → MCP. Agent identity resolved from headers, IP, fingerprint. All in-memory. Zero DB.

**Column 2:**
**STEP 2 — CIRCUIT BREAK (0ms)**
3 checks run before forwarding. Loop Detector: 5 identical tool calls per 60s → block. Risk Score: rolling avg > 75.0 over 5min → block. Kill Switch: agent status == "blocked" → 403. All in-memory HashMap. Zero DB round-trips.

**Column 3:**
**STEP 3 — FORWARD + CAPTURE (< 1ms)**
Request forwarded to real upstream API unchanged. Response streamed back to agent immediately. Event built from request+response metadata. `try_send()` to bounded channel — never blocks. Agent is done. Analysis happens async.

**Small note at bottom of Column 3:**
"We tried awaiting the DB write inline — agent requests blocked during DB slow periods. Moved to bounded channel. DB is now completely off the hot path."

---

### NEW SLIDE D — THE EVENT PROCESSING PIPELINE
**Use:** The "Key Competitive Advantages" layout — title left + texture panel right + 3 cards below

**Title area (left):** "Event Pipeline"
**Small text:** "Runs async. Off the hot path. After the agent already has its response."

**Decorative panel (right):** template's default texture — no text

**3 cards below:**

**Card 01 — EXTRACT**
Protocol parser reads model name, input/output token counts, tool call names, finish reason from raw JSON payload. OpenAI parser ≠ Anthropic parser ≠ MCP parser — each has its own schema mapping. Cost estimated from embedded pricing table (USD per 1M tokens, 8 decimal precision).

**Card 02 — ANALYSE**
PII scanner runs 5 compiled regex patterns (OnceLock — compiled once at startup, reused forever) in specificity order: SSN → CreditCard → Email → Phone → IP. Result: location + confidence score. Never the value itself. Lineage hash computed: SHA-256(prev_hash + event_id + agent_id + timestamp_ms). Compliance tag assigned.

**Card 03 — PERSIST**
Background writer drains channel in batches of 100 events every 100ms. Batch INSERT to TimescaleDB events hypertable (partitioned by timestamp, compressed after 7 days). Agent stats upserted. cost_daily materialized view refreshed. Anomaly detectors run on the batch. Alerts surfaced to dashboard.

**Small note below Card 03:**
"Why TimescaleDB over plain Postgres: 1M+ events/day caused sequential scans on time-range queries. TimescaleDB hypertable partitions by timestamp + compresses chunks after 7 days. Query time dropped 10×."

---

### NEW SLIDE E — THE CIRCUIT BREAKER SYSTEM
**Use:** The "Key Competitive Advantages" layout — title left + texture panel right + 3 cards below

**Title area (left):** "Circuit Breakers"
**Small text:** "Three in-memory checks. Zero DB. Always fail-open. Run before every upstream forward."

**Decorative panel (right):** template's default texture — no text

**3 cards below:**

**Card 01 — LOOP DETECTOR**
Detects agent tool-call loops before they cost money. Tracks call counts per (agent_id + tool_name) in a sliding 60-second window. Threshold: 5 identical calls → HTTP 429 returned to agent. State: in-memory HashMap with VecDeque timestamps. Resets on proxy restart. Prevents the $47,000 recursive loop.

**Card 02 — RISK CIRCUIT BREAKER**
Blocks agents whose rolling risk score exceeds threshold. Risk score computed as weighted rolling average across the last 5 minutes of events. Threshold: 75.0 (configurable). Block returns HTTP 503. State: in-memory per-agent score map. Updated on every event processed by the background writer.

**Card 03 — KILL SWITCH**
Manual override. Operator sets agent status = "blocked" via dashboard or REST API (PUT /api/v1/agents/{id}). Hot path checks this flag in-memory before forwarding. Next API call from that agent: HTTP 403. Instant. No redeployment. No code change. Block removed the same way.

---

### NEW SLIDE F — THE PII DETECTION ENGINE
**Use:** The "Introduction" split layout — left panel with text, right panel decorative/textured

**Left panel title:** "PII Detection"
**Left panel body:**

5 regex patterns compiled once at startup using Rust's `OnceLock<T>`:

  SSN         →  \d{3}-\d{2}-\d{4}      → replaces with [SSN]
  Credit Card →  \d{4}[-\s]?\d{4}...    → replaces with [CREDIT_CARD]
  Email       →  [a-zA-Z0-9._%+\-]+@... → replaces with [EMAIL]
  Phone       →  \d{3}[-.]?\d{3}[-.]?.. → replaces with [PHONE]
  IP Address  →  \d{1,3}\.\d{1,3}...    → replaces with [IP]

Applied in specificity order: most specific first (SSN, card), then broader patterns (phone, IP) — prevents overlapping digit sequences from double-matching.

Result logged: type + location (field path + offset). Never the matched value.
DPDP-compliant. GDPR-compliant. HIPAA-compliant.

**Small note below patterns:**
"First approach: compiled regex on every event. At 1000 RPS = 1000 compiles/sec. Switched to OnceLock — compiled once at startup, zero allocation on the analysis path."

**Right panel:** template decorative texture — no text

---

### NEW SLIDE G — THE ANOMALY DETECTION ENGINE
**Use:** The "Key Competitive Advantages" layout — title left + texture panel right + 3 cards below

**Title area (left):** "Anomaly Engine"
**Small text:** "Runs after DB flush. Never on the hot path. Fail-open — a crashing detector never stops the proxy."

**Decorative panel (right):** template's default texture — no text

**3 cards below:**

**Card 01 — OFF-HOURS DETECTOR**
Flags agents active outside 06:00–22:00 UTC. Every event timestamp checked against configurable business hours window. Alert fired with: agent_id, detected_at, hour of activity, severity=Warning. State: stateless — checks each event independently. Zero memory accumulation. Catches the "11 days running at 2AM" scenario.

**Card 02 — TOKEN VOLUME DETECTOR**
Tracks exponential moving average (EMA) of token counts per agent. Trigger: current event tokens > 5× EMA. Alert severity escalates: >5× = Warning, >10× = Critical. EMA adapts to agents that legitimately grow — no false positives on scaling agents. Catches the runaway loop that billed $47,000.

**Small note below Card 02:**
"Simple threshold (>N tokens) caused constant false positives on growing agents. Switched to EMA — adapts to the agent's own baseline. Alert only when 5× their own normal."

**Card 03 — NEW TOOL DETECTOR**
Tracks the set of tools each agent has ever called (per agent_id). Warm-up period: first 10 events ignored to let the baseline form. After warm-up: any tool_name not in the known set → Alert severity=Info. "data-pipeline agent just called delete_database_records for the first time" is the exact incident this catches. State: per-agent HashSet of seen tool names.

---

### NEW SLIDE H — DASHBOARD TOUR
**Use:** The "Discover Our Services" layout — card grid with titled cards

**Slide title:** "The Dashboard"
**Subtitle small:** "18 pages. React 18 + TypeScript. Live event stream. Full audit trail."

**Insert 4 screenshot images in a 2×2 grid:**

Screenshot tile 1 — label: **AUDIT**
(Insert screenshot of the events feed / live stream table)

Screenshot tile 2 — label: **COST**
(Insert screenshot of cost breakdown by agent/model)

Screenshot tile 3 — label: **ALERTS**
(Insert screenshot of anomaly alert panel)

Screenshot tile 4 — label: **CONTROL**
(Insert screenshot of agent detail page showing risk score + kill switch toggle)

**Bottom strip text:** "Self-hosted. Your data never leaves your infra."

---

## PLACEMENT INSTRUCTION FOR COPILOT

Insert these 8 new slides (A through H) **after the current Traction slide** and **before the Accomplishments/Roadmap slide**. The final slide order should be:

1. Title
2. Today's Agenda
3. Introduction ($47K incident)
4. Problem Statement
5. Our Innovative Solutions
6. Discover Our Capabilities
7. Size of Market
8. Direct/Indirect Competitor
9. Key Competitive Advantages
10. Traction
11. ← NEW: System Architecture Diagram
12. ← NEW: How It's Built (8 crates)
13. ← NEW: The Hot Path
14. ← NEW: Event Processing Pipeline
15. ← NEW: Circuit Breaker System
16. ← NEW: PII Detection Engine
17. ← NEW: Anomaly Detection Engine
18. ← NEW: Dashboard Tour
19. Accomplishments & Roadmap
20. Thank You

---

## ADDITIONAL INSTRUCTIONS

- Use the template's existing layouts exactly — do not introduce new slide masters
- For code snippets and text diagrams, use a monospace text box with small font — do not use code block formatting
- Keep all body text under 60 words per text box
- The tone is technical and direct — this is showing engineers how things actually work
- Use the template's accent color for highlighting key terms (the first word of each card title)
- Do not add slide numbers to the new slides manually — let the template handle it
- For the "Small note" lines in each card: use a smaller font size (10–11pt), italic, placed below the card body as a footer annotation

---

*End of prompt*
