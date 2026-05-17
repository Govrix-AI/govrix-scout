# Govrix Scout — Architecture Diagram (Full Slide, WHY-annotated)
### Go to mermaid.live → paste code → Actions → PNG → insert into PowerPoint full-bleed

---

## Mermaid Code — paste this exactly

```mermaid
flowchart LR
    %% ── AGENTS ───────────────────────────────────────────────────────────────
    subgraph Agents["🤖  AI AGENTS\nzero code changes  ·  one env var"]
        direction TB
        OA["OpenAI Agent"]
        AA["Anthropic Agent"]
        MA["MCP Agent"]
    end

    %% ── HOT PATH ─────────────────────────────────────────────────────────────
    subgraph Hot["⚡  HOT PATH — port 4000\nRust hyper  (NOT axum)\nWhy: axum routing adds ~300µs · hyper = raw bytes → <1ms p50"]
        direction TB
        ID["Identity\nheaders · IP · mTLS fingerprint\nresolves agent_id + session_id"]

        subgraph CB["CIRCUIT BREAKERS — all in-memory · zero DB on critical path"]
            direction LR
            LD["Loop Detector\nHashMap per (agent + tool)\n5 calls / 60s  →  HTTP 429\nprevents the $47K recursive loop"]
            RC["Risk Score\nrolling weighted avg per agent\n> 75.0 / 5 min  →  HTTP 503\nupdated async by background writer"]
            KS["Kill Switch\nagent.status == blocked  →  HTTP 403\n1-click in dashboard · instant · no redeploy"]
        end

        FW["Forward & Capture\nproxy request bytes unchanged\nstream response back to agent immediately\nbuild AgentEvent from metadata"]
    end

    BLK["⛔  BLOCKED\n429  ·  503  ·  403\nzero upstream tokens billed"]

    subgraph UP["☁️  UPSTREAM"]
        direction TB
        U1["OpenAI API"]
        U2["Anthropic API"]
        U3["MCP Server"]
    end

    %% ── BOUNDARY ─────────────────────────────────────────────────────────────
    CH["🔀  BOUNDED mpsc CHANNEL\ncapacity: 10,000 events\ntry_send()  →  never awaited · never blocks\nDB slow or down?  events drop · traffic flows\nThis line is the fail-open architectural guarantee"]

    %% ── ASYNC PIPELINE ───────────────────────────────────────────────────────
    subgraph Async["🔄  ASYNC PIPELINE — off hot path · background writer · 100 events / 100ms"]
        direction LR

        PII["PII Scanner\nOnceLock — compiled once at startup\nWhy OnceLock: 1000 RPS = 1000 compiles/sec otherwise\nSSN · Card · Email · Phone · IP\nSpecificity order: most specific first\nLogs location + offset  ·  never the value\n→ DPDP · GDPR · HIPAA"]

        COST["Cost Calculator\nUSD / 1M tokens per model\n8 decimal precision\nWhy 8 decimal: float rounds at millions of events\nper-event · per-session · per-agent-day"]

        MRK["Merkle Chain\nlineage_hash = SHA-256(\n  prev_hash + event_id\n  + agent_id + timestamp_ms)\nEdit any DB row →\nall downstream hashes break\nAuditor can verify independently"]

        ANO["Anomaly Detectors\nOff-Hours: 06:00–22:00 UTC · stateless\nToken EMA: >5× own avg → Warning\n  Why EMA not threshold:\n  adapts to legitimate agent growth\nNew Tool: HashSet + 10-event warmup\n  Why warmup: ignore init calls\nFail-open: crash here ≠ proxy stops"]
    end

    %% ── TIMESCALEDB ──────────────────────────────────────────────────────────
    subgraph DB["💾  TIMESCALEDB — not plain Postgres\nWhy: 1M+ events/day caused sequential scans on Postgres\nHypertable partitioned by timestamp → 10× faster time-range queries\nChunks compressed after 7 days → ~90% size reduction · 30-day auto-retention"]
        direction TB
        EV[("events hypertable\ntimestamp · agent_id · session_id\nlineage_hash · compliance_tag\ntokens · cost_usd · pii_detections")]
        AG[("agents · anomaly_alerts\ncost_daily continuous aggregate\nauto-refreshes on insert")]
    end

    %% ── OUTPUT ───────────────────────────────────────────────────────────────
    subgraph Out["📤  MANAGEMENT + OUTPUT"]
        direction TB
        API["REST API — port 4001 — axum · Bearer auth\n/events  ·  /agents  ·  /costs\n/reports  ·  /health  ·  /metrics\n/events/stream  (SSE live feed)"]
        RPT["Report Generator\nSOC 2 Type II · HIPAA\nEU AI Act Article 12+13 (Aug 2026)\nMerkle chain proof attached\nPDF + JSON evidence package"]
        DSH["📊 Dashboard — port 3000\nReact 18 · TanStack Query · SSE\n18 pages · live stream · kill switch\ncost · alerts · compliance reports"]
        PRM["Prometheus — port 9090\nevents_sent · events_dropped\nlatency_p50/p99 · cb_trips\npii_total · budget_utilization"]
    end

    %% ── CONNECTIONS ──────────────────────────────────────────────────────────
    OA & AA & MA -->|"HTTP  port 4000"| ID
    ID --> CB
    LD & RC & KS -->|"blocked"| BLK
    CB -->|"allowed"| FW
    FW <-->|"request / response"| U1 & U2 & U3
    FW -->|"response → agent\nbefore any DB write"| OA & AA & MA
    FW -->|"try_send()"| CH
    CH -->|"async"| PII & COST & MRK & ANO
    PII & COST & MRK --> EV
    ANO --> AG
    EV & AG --> API
    API --> RPT & DSH & PRM

    %% ── STYLES ───────────────────────────────────────────────────────────────
    style Agents fill:#0d1824,stroke:#4a9eff,stroke-width:2px,color:#cce4ff
    style Hot    fill:#0a1628,stroke:#4a9eff,stroke-width:3px,color:#cce4ff
    style CB     fill:#12103a,stroke:#7b68ee,stroke-width:2px,color:#d8d4ff
    style Async  fill:#0a1e10,stroke:#2ecc71,stroke-width:3px,color:#c4f0d0
    style DB     fill:#160d28,stroke:#9b59b6,stroke-width:3px,color:#e8d4ff
    style Out    fill:#1e1006,stroke:#e67e22,stroke-width:3px,color:#ffe0b2
    style UP     fill:#0d1824,stroke:#4a9eff,stroke-width:1px,color:#cce4ff

    style OA  fill:#0d1824,stroke:#4a9eff,color:#cce4ff
    style AA  fill:#0d1824,stroke:#4a9eff,color:#cce4ff
    style MA  fill:#0d1824,stroke:#4a9eff,color:#cce4ff
    style U1  fill:#0d1824,stroke:#4a9eff,color:#cce4ff
    style U2  fill:#0d1824,stroke:#4a9eff,color:#cce4ff
    style U3  fill:#0d1824,stroke:#4a9eff,color:#cce4ff

    style ID  fill:#0a1628,stroke:#4a9eff,color:#cce4ff
    style FW  fill:#0a1628,stroke:#4a9eff,color:#cce4ff
    style LD  fill:#12103a,stroke:#7b68ee,color:#d8d4ff
    style RC  fill:#12103a,stroke:#7b68ee,color:#d8d4ff
    style KS  fill:#12103a,stroke:#7b68ee,color:#d8d4ff

    style CH  fill:#0a1e10,stroke:#2ecc71,stroke-width:4px,color:#c4f0d0
    style BLK fill:#280a0a,stroke:#e74c3c,stroke-width:2px,color:#ffcccc

    style PII  fill:#071a0b,stroke:#27ae60,color:#c4f0d0
    style COST fill:#071a0b,stroke:#27ae60,color:#c4f0d0
    style MRK  fill:#071a0b,stroke:#27ae60,color:#c4f0d0
    style ANO  fill:#071a0b,stroke:#27ae60,color:#c4f0d0

    style EV  fill:#160d28,stroke:#9b59b6,color:#e8d4ff
    style AG  fill:#160d28,stroke:#9b59b6,color:#e8d4ff

    style API fill:#1e1006,stroke:#e67e22,color:#ffe0b2
    style RPT fill:#1e1006,stroke:#e67e22,color:#ffe0b2
    style DSH fill:#1a1a06,stroke:#f1c40f,color:#fffacc
    style PRM fill:#1e1006,stroke:#e67e22,color:#ffe0b2
```

---

## Export settings for best PPT quality

1. In mermaid.live top-right: click **Actions → PNG**
2. Before downloading, set **Scale: 3** (or highest available) — this gives crisp resolution on a projector
3. Save as `architecture-diagram.png`

---

## Insert into PowerPoint

- Slide layout: **blank slide** (no title — the diagram is the whole slide)
- Insert → Pictures → This Device → select the PNG
- Drag to fill **edge-to-edge** — no white margins
- The diagram is already dark background — it will look native on the dark template

OR use the split layout:
- Left panel: title text "System Architecture" + 30-second talk track
- Right panel: insert the PNG

---

## Colour legend (say this in 10 seconds)

> "Blue is the hot path — synchronous, in-memory, under 1ms.
> Purple is the circuit breakers — they stop bad agents before a single token is billed.
> The green channel is the fail-open boundary — once I send the response, everything else is async.
> Green boxes are the analysis pipeline — PII, cost, Merkle, anomaly — all off the critical path.
> Purple database is TimescaleDB — chosen over plain Postgres for 10× faster time queries.
> Orange is the management API and reports.
> Yellow is the dashboard."

---

## 30-second verbal walkthrough for this slide

> "Agent comes in on port 4000. Three in-memory circuit breakers check it — loop detection, risk score, kill switch. If any fires, the agent gets a 4xx and zero tokens are billed. If it passes, the request goes to OpenAI or Anthropic unchanged. The response comes back to the agent immediately — before we write a single byte to the database. That's the green channel — bounded, non-blocking, fail-open. Even if the database is down, the agent never knows. The async pipeline picks up the event after the agent is done: PII scanning, cost calculation, a SHA-256 Merkle chain for tamper-evidence, and three anomaly detectors. Everything lands in TimescaleDB — we switched from plain Postgres because at a million events a day, time-range queries were sequential scans. The management API serves the dashboard, compliance reports, and Prometheus. That's the whole system."

---

*End of diagram guide*
