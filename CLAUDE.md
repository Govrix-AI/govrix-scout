# AgentMesh — Project Configuration

## FIRST: Read Shared Context

**EVERY Claude session MUST read these files before doing ANY work:**
1. `.context/SESSION_LOG.md` — What was done, what's left, blockers (append-only)
2. `.context/MEMORY.md` — Stable reference (tech stack, schemas, architecture)

**After EVERY session, update `.context/SESSION_LOG.md`** with:
- What was done (numbered list)
- What's left (checkbox list)
- Test counts
- Key decisions made
- Blockers/notes

This is how multiple Claude instances stay in sync across sessions and machines.

## Monorepo Structure

```
/home/manas/Code/Startup/
├── .context/                      # SHARED CONTEXT (read first every session!)
│   ├── MEMORY.md                  # Stable project knowledge
│   └── SESSION_LOG.md             # Per-session progress log
├── Docs/                          # Documentation & specs
│   ├── Tech/                      # Technical build specs
│   ├── Ideation/                  # Product ideation docs
│   └── MarketResearch/            # Market research
├── Opensource/                     # AgentMesh OSS
│   └── agentmesh/                 # Rust proxy + React dashboard monorepo
├── Scanner/                       # Agent scanning components
├── .claude/
│   └── skills/                    # Claude Code skills (MUST use)
│       ├── rust-proxy/            # Rust proxy architecture decisions
│       ├── agentmesh-schemas/     # Canonical DB schemas (PG, CH, SQLite)
│       └── compliance-first/      # Core compliance invariant
└── CLAUDE.md                      # This file
```

## Tech Stack

| Layer | Technology | Where |
|-------|-----------|-------|
| **SaaS Proxy** | Rust (hyper, not axum for hot path) | Future SaaS platform |
| **SaaS Backend** | TypeScript + Next.js | Future SaaS platform |
| **SaaS Storage** | PostgreSQL (registry) + ClickHouse (events) | Future SaaS platform |
| **Scout OSS** | Python 3.11+ (Click, FastAPI, HTMX, SQLite) | `Opensource/agentmesh-scout/` |
| **Scout Storage** | SQLite + SQLAlchemy 2.0+ | Local only |
| **Scout Dashboard** | FastAPI + HTMX + Tailwind CSS | localhost:8477 |
| **PII Detection** | Microsoft Presidio + regex | Both OSS and SaaS |
| **Reports** | ReportLab (PDF) + JSON | Both OSS and SaaS |

## Skills (MUST Read Before Coding)

These skills encode architecture decisions. Every subagent MUST invoke the relevant skill before writing code:

- **rust-proxy** — Use when touching proxy/networking code. Encodes: hyper not axum, SSE stream-through, body tee pattern, `detect_protocol()` signature, fail-open design.
- **agentmesh-schemas** — Use when touching ANY database code. Contains canonical SQL for PostgreSQL (agent registry), ClickHouse (event log), and SQLite (Scout). Table names, column types, indexes are final.
- **compliance-first** — Use when writing ANY interceptor/logging code. Core invariant: every intercepted action MUST generate `session_id`, `timestamp`, `lineage_hash`, `compliance_tag`. No exceptions.

## Inter-Agent Communication

- **MEMORY.md** (`~/.claude/projects/-home-manas-Code-Startup/memory/MEMORY.md`) is the persistent memory file for cross-session knowledge
- Subagents should record architectural decisions and patterns discovered during work
- Check MEMORY.md at session start for context from previous sessions

## Development Rules

### Always Use Subagents for File Reading
When exploring the codebase or reading multiple files, use the Task tool with `subagent_type=Explore` rather than reading files sequentially in the main context. This protects the main context window.

### Scout OSS Constraints (from Build Spec)
1. **Zero agent modification** — Agents connect by changing ONE env var, no code changes
2. **Local-only data** — No telemetry, no cloud sync, SQLite only, localhost dashboard
3. **Fail-open proxy** — If proxy crashes, agent traffic continues
4. **Latency budget** — Proxy adds < 5ms per request; analysis is async
5. **No PII in alerts** — Store type and location, NEVER the actual PII value
6. **Read-only observation** — Scout NEVER modifies/blocks/filters agent traffic
7. **Apache 2.0 license** — Permissive, no copyleft
8. **Python 3.11+ only** — Use modern features (match, tomllib, improved asyncio)
9. **Upsell placement** — PDF report footer and dashboard footer link to AgentMesh SaaS

### SaaS Platform Constraints
1. **Rust proxy** — Follow rust-proxy skill exactly for hot path decisions
2. **Compliance-first** — Every event gets the four compliance fields (see skill)
3. **Schema fidelity** — All DB code matches agentmesh-schemas skill exactly

### Code Quality
- Use `black` + `ruff` for Python formatting/linting
- Use `clippy` for Rust linting
- Every module must have corresponding tests (pytest for Python, cargo test for Rust)
- No hardcoded API keys, tokens, or secrets anywhere

## Build Spec Reference

The canonical build specification for Scout OSS is at:
`Docs/Tech/AgentMesh_Scout_Build_Spec.docx`

This document defines: project structure, all CLI commands, database schema, proxy implementation, PII detection spec, dashboard pages, build phases, testing strategy, and launch checklist.
