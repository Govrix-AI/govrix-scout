# Microsoft Copilot — PowerPoint Generation Prompt
### Paste everything below the dashed line into Microsoft Copilot for PowerPoint

---

## PROMPT — COPY FROM HERE

---

I have already applied a **"White, Blue and Grey Modern Startup Pitch Deck"** template to this presentation. Use the template's existing colors, fonts, and styling exactly as-is for every slide — do not change the theme. Your job is only to fill in the content and match the layout pattern described for each slide. 12 slides total.

**General rules:**
- Use the template's font as-is — do not change typeface or weights
- Use the template's color scheme as-is — do not override with custom colors
- Every slide has a small brand label top-left: "govrix scout" — match the template's header label style
- Slide numbers bottom-right — match template style
- Maximum 50 words of body text per slide — cut ruthlessly, one idea per slide
- No clip art, no stock photo placeholders — use shapes and text hierarchy only
- Do not add decorative borders not present in the template
- The tone is direct and confident — no filler phrases like "In conclusion" or "As we can see"

---

### SLIDE 1 — TITLE
**Use:** The template's title/cover slide layout

**Content:**
- Main headline (large, bold, dominant): **"Govrix Scout."** — include the period
- Subtitle below: "A transparent governance proxy for every AI agent call"
- Bottom-left: [Your Name] | B.Tech CSE | [College Name]
- Bottom-right: github.com/manaspros/govrix-scout

---

### SLIDE 2 — TODAY'S AGENDA
**Use:** The template's agenda slide layout — left stacked title, right numbered list

**Content:**
- Left: "Today's" (regular) then "Agenda" (bold, large) — stacked
- Right numbered list:
  1. The Problem — Crime scene, not control room
  2. Why Ownership Matters
  3. Govrix Scout — What it is
  4. Key Capabilities
  5. Market & Competitive Landscape
  6. Traction & Roadmap

---

### SLIDE 3 — INTRODUCTION (THE INCIDENT)
**Use:** The template's introduction split layout — left panel with text, right panel decorative/textured

**Content:**
- Left panel top label (small): "November 2025"
- Left panel main text (large, bold): **"$47,000."**
- Left panel body: "Two AI agents. Talking to each other. For 11 days. Nobody noticed. The logs existed. Nobody was watching."
- Left panel bottom italic: "That's not a horror story. That's the default state."
- Right panel: decorative only — no text, use template's default right-panel treatment

---

### SLIDE 4 — PROBLEM STATEMENT
**Use:** The template's problem statement layout — full-width header, 3 equal columns below with icon + text

**Content:**
- Header: **"Problem Statement"**
- Column 1:
  - Title: **Crime Scene, Not Control Room**
  - Body: LangSmith. Portkey. Helicone. Logs exist. Nobody reads them until production burns. Reactive by design.
- Column 2:
  - Title: **You Don't Own Your Infra**
  - Body: Your agent's prompts, costs, and compliance evidence sit on a SaaS vendor's servers. You rent visibility into your own operations.
- Column 3:
  - Title: **Tools Log. They Don't Think.**
  - Body: No proactive alerts. No pattern detection. A $47K loop ran for 11 days. Nobody was notified.

---

### SLIDE 5 — OUR SOLUTION
**Use:** The template's "Our Innovative Solutions" split layout — left title panel, right content panel

**Content:**
- Left panel title (bold, stacked):
  "Our"
  "Innovative"
  "Solution"
- Left panel small text below: "Zero agent code changes."
- Right panel — flow diagram:
  [Your Agent] ——► [GOVRIX SCOUT] ——► [OpenAI / Anthropic / MCP]
  Arrow down to box: "Every request. Every response. Every token. Tamper-evident. On your infra."
  Small monospace text: export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1
  Below: "One line. Done."

---

### SLIDE 6 — DISCOVER OUR CAPABILITIES
**Use:** The template's "Discover Our Services" layout — card grid with titled cards

**Content:**
- Slide header: **"Discover Our Capabilities"**
- 4 cards in a 2×2 grid:

  Card 1 — **PII DETECTION**
  Real-time. Every request scanned. EMAIL · PHONE · SSN · CREDIT CARD · IP. Logs location, never the value. DPDP + GDPR compliant.

  Card 2 — **ANOMALY ALERTS**
  Proactive, not reactive. Off-hours usage. Token spikes (11× baseline). New tool invoked. Alert fires — not a Slack audit 3 weeks later.

  Card 3 — **AUTO MODEL ROUTING**
  Simple query → gpt-4o-mini. Complex reasoning → gpt-4o. Rust-speed decision. LiteLLM caps at 500 RPS. Govrix Scout doesn't.

  Card 4 — **KILL SWITCH + BUDGET**
  Block any agent in one click. 403 before it reaches the API. Daily/monthly budget caps auto-enforce across your entire fleet.

---

### SLIDE 7 — SIZE OF MARKET
**Use:** The template's "Size of Market" layout — left title area, right large dominant stat number

**Content:**
- Left: "Size of" / "Market" — bold stacked
- Left body small: "AI governance is Gartner's newest billion-dollar category."
- Right dominant stat (very large, bold): **"2.52 T"**
- Right sub-label: "Worldwide AI spending in 2026 — Gartner"
- Circle element: **"492 M"** — label: "AI governance market 2026"
- Bottom strip: "95% of executives call sovereign AI infrastructure mission-critical within 3 years — DataCenterPost 2025"

---

### SLIDE 8 — DIRECT + INDIRECT COMPETITOR
**Use:** The template's "Direct Competitor / Indirect Competitor" two-column layout

**Content:**
- Left column header: **"Direct Competitor"**
- Left column body:
  — LangSmith (LangChain only, no tamper trail)
  — Portkey (SaaS-first, your data on their servers)
  — Helicone (no anomaly alerts, no kill switch)
  — Cisco AI Defense (network layer, not app layer)

- Right column header: **"Indirect Competitor"**
- Right column body:
  — LiteLLM (Python, 500 RPS wall, not compliance-grade)
  — Datadog / New Relic (infra observability, not AI-specific)
  — Manual logging (no pattern detection, reactive only)
  — Nothing (the real competitor — most orgs have nothing)

- Bottom full-width note: "The gap no one fills: self-hosted · tamper-evident · sub-millisecond · proactive alerts · MCP support"

---

### SLIDE 9 — KEY COMPETITIVE ADVANTAGES
**Use:** The template's "Key Competitive Advantages" layout — title area top, textured panel, 3 advantage cards below

**Content:**
- Title area: "Key" / "Competitive" / "Advantages" — bold stacked
- Small text: "Built in Rust. Designed for compliance. Owned by you."
- Decorative panel: use template's default treatment (no added text)
- 3 cards below (Advantage 01 / 02 / 03 style):

  **Advantage 01 — RUST HOT PATH**
  "<1ms p50 latency. No GC pauses. No Python GIL. LiteLLM caps at 500 RPS. Govrix Scout: same speed at 500 or 50,000 RPS."

  **Advantage 02 — MERKLE CHAIN AUDIT**
  "SHA-256 cryptographic chain on every event. Modify one database record — every downstream hash breaks. Mathematically tamper-proof."

  **Advantage 03 — FAIL-OPEN DESIGN**
  "Database down? Proxy runs. Channel full? Events drop, traffic flows. Internal errors never become your production incident."

---

### SLIDE 10 — TRACTION
**Use:** The template's "Traction" layout — left title panel, right chart/data panel

**Content:**
- Left panel title: **"Traction"**
- Left panel checklist:
  ✓ Proxy live — <1ms p50
  ✓ OpenAI + Anthropic + MCP
  ✓ 3 anomaly detectors active
  ✓ PII scanning every event
  ✓ 18-page React dashboard
  ✓ TimescaleDB persisting events
  ✓ Merkle chain intact on all writes
  ✓ Open source on GitHub
- Right panel: line chart — X axis: Oct 2024 → Apr 2026, Y axis: capabilities shipped, curve rises with milestone dots
- Right panel bottom small: "docker compose up → 1 env var → governed. Your data stays on your infra."
- Right panel label pill: "5 Rust crates · CI/CD · fully tested"

---

### SLIDE 11 — ACCOMPLISHMENTS & ROADMAP
**Use:** The template's "Accomplishments Date" timeline layout — horizontal timeline with circle milestones and labels

**Content:**
- Header: **"Accomplishments & Roadmap"**
- Horizontal timeline with 4 milestone circles:

  Circle 1 — **"NOW"**
  Wire-level proxy · PII detection · 3 anomaly detectors · Cost tracking · Kill switch · Merkle chain · 18-page dashboard · Open source

  Circle 2 — **"90 Days"**
  Policy enforcement in hot path · PII masking · 50+ PII types · ML anomaly baselines · 1-click bulk changes · Slack/PagerDuty · Kubernetes operator

  Circle 3 — **"12 Months"**
  Multi-agent conversation graphs · Prompt injection detection · Compliance reports (SOC 2, HIPAA, EU AI Act, NIST)

  Circle 4 — **"Vision"**
  Every AI agent call goes through a governance proxy. The way every packet goes through a firewall.

---

### SLIDE 12 — THANK YOU
**Use:** The template's closing "Thank You" slide layout

**Content:**
- Main text (large, bold, centered): **"Thank You."** — include the period
- Below (centered, regular):
  "Every packet goes through a proxy."
  "Every AI agent call will too."
  "Govrix Scout is building that layer."
- Divider line
- Below divider (centered, small):
  Open source  ·  Self-hosted  ·  Your data  ·  Your infra  ·  Your control
- Bottom-right: github.com/manaspros/govrix-scout

---

*End of prompt*
