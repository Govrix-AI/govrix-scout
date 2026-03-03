# Dashboard TypeScript Rewrite — Design Doc
Date: 2026-03-03

## Summary
Replace the existing glassmorphism TypeScript dashboard with a new TypeScript dashboard
that matches the govrix enterprise dashboard's visual design. OSS pages are wired to the
18 real govrix-scout backend endpoints. Platform-only pages are locked behind an
EnterpriseGate upgrade card.

## Scope

### Remove
- Entire `dashboard/src/` (old glassmorphism TS dashboard)

### Build (TypeScript)
- `App.tsx` — Router, EnterpriseGate, AuthContext provider
- `context/AuthContext.tsx` — tier state ("Scout" | "Platform")
- `api/client.ts` — fetch wrapper, base URL from `VITE_API_URL` env (default `http://localhost:4001`)
- `api/types.ts` — typed interfaces for all 18 backend response shapes
- `api/hooks.ts` — TanStack Query v5 hooks per endpoint
- `components/layout/` — Sidebar, Header, Layout
- `components/common/` — StatusBadge, DataTable, EmptyState, LoadingState, TimeRangePicker
- `pages/OverviewPage.tsx` — health + cost summary + agent count KPIs
- `pages/EventsPage.tsx` — paginated events list + detail modal
- `pages/AgentsPage.tsx` — agent list, update, retire actions
- `pages/CostsPage.tsx` — cost summary + breakdown by model/agent
- `pages/ReportsPage.tsx` — list reports + generate new report
- `pages/GatedPage.tsx` — EnterpriseGate upgrade card (lock icon + CTA)

## Page → Endpoint Map

| Page | Endpoints used |
|------|---------------|
| Overview | `GET /health`, `GET /api/v1/costs/summary`, `GET /api/v1/agents` (count) |
| Events | `GET /api/v1/events`, `GET /api/v1/events/{id}`, `GET /api/v1/events/sessions/{session_id}` |
| Agents | `GET /api/v1/agents`, `GET /api/v1/agents/{id}`, `PUT /api/v1/agents/{id}`, `POST /api/v1/agents/{id}/retire`, `GET /api/v1/agents/{id}/events` |
| Costs | `GET /api/v1/costs/summary`, `GET /api/v1/costs/breakdown` |
| Reports | `GET /api/v1/reports/types`, `GET /api/v1/reports`, `POST /api/v1/reports/generate` |
| Sessions / Governance / Compliance / Settings | EnterpriseGate (no backend calls) |

## Gated Pages (EnterpriseGate)
- `/sessions` — real-time session replay
- `/governance` — risk posture + alerts
- `/compliance` — compliance framework reports
- `/settings` — enterprise config

EnterpriseGate renders: lock icon, "Platform Feature" heading, upgrade message, CTA to govrix.io/platform.

## Visual Design
Match govrix enterprise dashboard: Tailwind CSS, dark/light mode, sidebar nav with locked icons on gated pages, sticky header.

## Non-Goals
- No mock/stub data for enterprise endpoints in scout
- No JSX — TypeScript throughout
- No new backend endpoints — wire only what exists
