# Scout Dashboard TypeScript Rewrite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the glassmorphism OSS dashboard with a new TypeScript dashboard matching the govrix enterprise visual design — OSS pages wired to the 18 real backend endpoints, enterprise pages locked behind EnterpriseGate.

**Architecture:** Vite + React 18 + TypeScript. TanStack Query v5 for all data fetching. AuthContext holds tier state. EnterpriseGate wraps platform-only routes. API layer typed to govrix-scout's 18 backend endpoints on port 4001.

**Tech Stack:** React 18, TypeScript, Vite, TanStack Query v5, React Router v6, Recharts, Tailwind CSS, Lucide React

---

### Task 1: Fix vite proxy and wipe old src

**Files:**
- Modify: `dashboard/vite.config.ts`
- Delete: `dashboard/src/` (entire directory — will be replaced)

**Step 1: Update vite proxy from wrong port 8080 → 4001**

Replace the entire contents of `dashboard/vite.config.ts`:
```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    proxy: {
      '/api': { target: 'http://localhost:4001', changeOrigin: true },
      '/health': { target: 'http://localhost:4001', changeOrigin: true },
      '/ready': { target: 'http://localhost:4001', changeOrigin: true },
      '/metrics': { target: 'http://localhost:4001', changeOrigin: true },
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom', 'react-router-dom'],
          query: ['@tanstack/react-query'],
          charts: ['recharts'],
        },
      },
    },
  },
})
```

**Step 2: Remove old src directory**
```bash
rm -rf /Users/manas.choudhary/Documents/Project/govrix/govrix-scout/dashboard/src
mkdir -p /Users/manas.choudhary/Documents/Project/govrix/govrix-scout/dashboard/src/{api,context,components/layout,components/common,pages}
```

**Step 3: Commit**
```bash
cd /Users/manas.choudhary/Documents/Project/govrix/govrix-scout
git add dashboard/vite.config.ts
git add -u dashboard/src/
git commit -m "chore(dashboard): fix vite proxy port, remove old dashboard src"
```

---

### Task 2: API types

**Files:**
- Create: `dashboard/src/api/types.ts`

**Step 1: Write complete types matching all 18 govrix-scout backend endpoints**

```ts
// dashboard/src/api/types.ts

export interface HealthResponse {
  status: 'ok' | 'degraded' | 'error'
  version: string
  uptime_secs: number
}

export interface AgentEvent {
  id: string
  session_id: string
  agent_id: string
  timestamp: string
  kind: string
  protocol: string
  model: string | null
  provider: string | null
  input_tokens: number | null
  output_tokens: number | null
  cost_usd: number | null
  latency_ms: number | null
  status_code: number | null
  pii_detected: boolean
  compliance_tag: string
  lineage_hash: string
  request_body: string | null
  response_body: string | null
}

export interface Agent {
  id: string
  name: string
  status: 'active' | 'retired' | 'blocked'
  first_seen: string
  last_seen: string
  total_requests: number
  total_cost_usd: number
  total_input_tokens: number
  total_output_tokens: number
}

export interface CostSummary {
  total_cost_usd: number
  total_requests: number
  total_input_tokens: number
  total_output_tokens: number
  avg_cost_per_request: number
  period_start: string
  period_end: string
}

export interface CostBucket {
  label: string
  cost_usd: number
  requests: number
  input_tokens: number
  output_tokens: number
}

export interface CostBreakdown {
  by_model: CostBucket[]
  by_agent: CostBucket[]
  by_provider: CostBucket[]
}

export interface ReportType {
  id: string
  name: string
  description: string
}

export interface Report {
  id: string
  report_type: string
  status: 'pending' | 'complete' | 'failed'
  created_at: string
  download_url: string | null
}

export interface GenerateReportRequest {
  report_type: string
  format: 'pdf' | 'json' | 'csv'
  start_date?: string
  end_date?: string
}

export interface SystemConfig {
  proxy_port: number
  management_port: number
  max_agents: number
  retention_days: number
  pii_detection_enabled: boolean
  budget_enforcement_enabled: boolean
}

export interface PaginatedResponse<T> {
  data: T[]
  total: number
}

export interface EventFilters {
  agent_id?: string
  session_id?: string
  kind?: string
  limit?: number
  offset?: number
}
```

**Step 2: Commit**
```bash
cd /Users/manas.choudhary/Documents/Project/govrix/govrix-scout
git add dashboard/src/api/types.ts
git commit -m "feat(dashboard): add typed API interfaces for all 18 scout endpoints"
```

---

### Task 3: API client

**Files:**
- Create: `dashboard/src/api/client.ts`

**Step 1: Write typed fetch wrapper**

```ts
// dashboard/src/api/client.ts
import type {
  AgentEvent, Agent, CostSummary, CostBreakdown,
  HealthResponse, Report, ReportType, GenerateReportRequest,
  SystemConfig, PaginatedResponse, EventFilters,
} from './types'

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
  })
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText)
    throw new ApiError(res.status, text)
  }
  return res.json() as Promise<T>
}

function buildParams(filters: Record<string, string | number | boolean | undefined>): string {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(filters)) {
    if (v !== undefined && v !== null) p.set(k, String(v))
  }
  const s = p.toString()
  return s ? `?${s}` : ''
}

// Health
export const getHealth = () => request<HealthResponse>('/health')

// Events
export const getEvents = (filters: EventFilters = {}) =>
  request<PaginatedResponse<AgentEvent>>(`/api/v1/events${buildParams(filters)}`)
export const getEvent = (id: string) =>
  request<AgentEvent>(`/api/v1/events/${id}`)
export const getSessionEvents = (sessionId: string) =>
  request<PaginatedResponse<AgentEvent>>(`/api/v1/events/sessions/${sessionId}`)

// Agents
export const getAgents = () =>
  request<PaginatedResponse<Agent>>('/api/v1/agents')
export const getAgent = (id: string) =>
  request<Agent>(`/api/v1/agents/${id}`)
export const updateAgent = (id: string, body: Partial<Pick<Agent, 'name' | 'status'>>) =>
  request<Agent>(`/api/v1/agents/${id}`, { method: 'PUT', body: JSON.stringify(body) })
export const retireAgent = (id: string) =>
  request<void>(`/api/v1/agents/${id}/retire`, { method: 'POST' })
export const getAgentEvents = (id: string, filters: EventFilters = {}) =>
  request<PaginatedResponse<AgentEvent>>(`/api/v1/agents/${id}/events${buildParams(filters)}`)

// Costs
export const getCostSummary = () =>
  request<CostSummary>('/api/v1/costs/summary')
export const getCostBreakdown = () =>
  request<CostBreakdown>('/api/v1/costs/breakdown')

// Reports
export const getReportTypes = () =>
  request<PaginatedResponse<ReportType>>('/api/v1/reports/types')
export const getReports = () =>
  request<PaginatedResponse<Report>>('/api/v1/reports')
export const generateReport = (body: GenerateReportRequest) =>
  request<Report>('/api/v1/reports/generate', { method: 'POST', body: JSON.stringify(body) })

// Config
export const getConfig = () =>
  request<SystemConfig>('/api/v1/config')
```

**Step 2: Commit**
```bash
git add dashboard/src/api/client.ts
git commit -m "feat(dashboard): add type-safe API client for all scout endpoints"
```

---

### Task 4: TanStack Query hooks

**Files:**
- Create: `dashboard/src/api/hooks.ts`

**Step 1: Write hooks for every endpoint**

```ts
// dashboard/src/api/hooks.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import * as api from './client'
import type { EventFilters, GenerateReportRequest } from './types'

export const qk = {
  health: ['health'] as const,
  events: (f: EventFilters) => ['events', f] as const,
  event: (id: string) => ['events', id] as const,
  sessionEvents: (sid: string) => ['events', 'session', sid] as const,
  agents: ['agents'] as const,
  agent: (id: string) => ['agents', id] as const,
  agentEvents: (id: string, f: EventFilters) => ['agents', id, 'events', f] as const,
  costSummary: ['costs', 'summary'] as const,
  costBreakdown: ['costs', 'breakdown'] as const,
  reportTypes: ['reports', 'types'] as const,
  reports: ['reports'] as const,
  config: ['config'] as const,
}

// Health — live, poll every 10s
export const useHealth = () =>
  useQuery({ queryKey: qk.health, queryFn: api.getHealth, refetchInterval: 10_000 })

// Events — poll every 5s
export const useEvents = (filters: EventFilters = {}) =>
  useQuery({ queryKey: qk.events(filters), queryFn: () => api.getEvents(filters), refetchInterval: 5_000 })

export const useEvent = (id: string) =>
  useQuery({ queryKey: qk.event(id), queryFn: () => api.getEvent(id), enabled: !!id })

export const useSessionEvents = (sessionId: string) =>
  useQuery({ queryKey: qk.sessionEvents(sessionId), queryFn: () => api.getSessionEvents(sessionId), enabled: !!sessionId })

// Agents — poll every 10s
export const useAgents = () =>
  useQuery({ queryKey: qk.agents, queryFn: api.getAgents, refetchInterval: 10_000 })

export const useAgent = (id: string) =>
  useQuery({ queryKey: qk.agent(id), queryFn: () => api.getAgent(id), enabled: !!id })

export const useAgentEvents = (id: string, filters: EventFilters = {}) =>
  useQuery({ queryKey: qk.agentEvents(id, filters), queryFn: () => api.getAgentEvents(id, filters), enabled: !!id })

export const useUpdateAgent = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: Parameters<typeof api.updateAgent>[1] }) =>
      api.updateAgent(id, body),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.agents }) },
  })
}

export const useRetireAgent = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.retireAgent(id),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.agents }) },
  })
}

// Costs — stable, refetch every 30s
export const useCostSummary = () =>
  useQuery({ queryKey: qk.costSummary, queryFn: api.getCostSummary, refetchInterval: 30_000 })

export const useCostBreakdown = () =>
  useQuery({ queryKey: qk.costBreakdown, queryFn: api.getCostBreakdown, refetchInterval: 30_000 })

// Reports
export const useReportTypes = () =>
  useQuery({ queryKey: qk.reportTypes, queryFn: api.getReportTypes, staleTime: Infinity })

export const useReports = () =>
  useQuery({ queryKey: qk.reports, queryFn: api.getReports, refetchInterval: 15_000 })

export const useGenerateReport = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: GenerateReportRequest) => api.generateReport(body),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.reports }) },
  })
}

// Config — stable, no polling
export const useConfig = () =>
  useQuery({ queryKey: qk.config, queryFn: api.getConfig, staleTime: 60_000 })
```

**Step 2: Commit**
```bash
git add dashboard/src/api/hooks.ts
git commit -m "feat(dashboard): add TanStack Query hooks for all scout endpoints"
```

---

### Task 5: AuthContext + app entry

**Files:**
- Create: `dashboard/src/context/AuthContext.tsx`
- Create: `dashboard/src/main.tsx`
- Create: `dashboard/src/index.css`

**Step 1: AuthContext**

```tsx
// dashboard/src/context/AuthContext.tsx
import { createContext, useContext, useState, type ReactNode } from 'react'

type Tier = 'Scout' | 'Platform'

interface AuthContextValue {
  tier: Tier
  darkMode: boolean
  toggleDarkMode: () => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [darkMode, setDarkMode] = useState(() => {
    const saved = localStorage.getItem('govrix-dark-mode')
    return saved ? saved === 'true' : false
  })

  const toggleDarkMode = () => {
    setDarkMode(d => {
      localStorage.setItem('govrix-dark-mode', String(!d))
      return !d
    })
  }

  return (
    <AuthContext.Provider value={{ tier: 'Scout', darkMode, toggleDarkMode }}>
      <div className={darkMode ? 'dark' : ''}>{children}</div>
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used inside AuthProvider')
  return ctx
}
```

**Step 2: main.tsx**

```tsx
// dashboard/src/main.tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import './index.css'
import App from './App'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 2, refetchOnWindowFocus: false },
  },
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>
)
```

**Step 3: index.css** — Keep Tailwind directives, add dark mode variables:

```css
/* dashboard/src/index.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    --bg: 248 250 252;
    --surface: 255 255 255;
    --border: 226 232 240;
    --text-primary: 15 23 42;
    --text-secondary: 100 116 139;
  }
  .dark {
    --bg: 10 10 15;
    --surface: 17 17 27;
    --border: 39 39 55;
    --text-primary: 241 245 249;
    --text-secondary: 148 163 184;
  }
  body {
    @apply bg-slate-50 dark:bg-[#0a0a0f] text-slate-900 dark:text-slate-100 font-sans antialiased;
  }
}

@layer components {
  .btn-primary {
    @apply bg-indigo-600 hover:bg-indigo-700 text-white font-medium px-4 py-2 rounded-lg transition-colors;
  }
  .card {
    @apply bg-white dark:bg-[#11111b] border border-slate-200 dark:border-[#272737] rounded-xl p-5;
  }
  .stat-card {
    @apply card flex flex-col gap-1;
  }
}
```

**Step 4: Commit**
```bash
git add dashboard/src/context/AuthContext.tsx dashboard/src/main.tsx dashboard/src/index.css
git commit -m "feat(dashboard): add AuthContext, main entry, CSS base"
```

---

### Task 6: Layout components (Sidebar, Header, Layout)

**Files:**
- Create: `dashboard/src/components/layout/Sidebar.tsx`
- Create: `dashboard/src/components/layout/Header.tsx`
- Create: `dashboard/src/components/layout/Layout.tsx`

**Step 1: Sidebar.tsx**

```tsx
// dashboard/src/components/layout/Sidebar.tsx
import { NavLink } from 'react-router-dom'
import { LayoutDashboard, Zap, Users, DollarSign, FileText, Shield, Video, CheckSquare, Settings, Lock } from 'lucide-react'
import { useAuth } from '../../context/AuthContext'

const OSS_NAV = [
  { to: '/overview', icon: LayoutDashboard, label: 'Overview' },
  { to: '/events', icon: Zap, label: 'Events' },
  { to: '/agents', icon: Users, label: 'Agents' },
  { to: '/costs', icon: DollarSign, label: 'Costs' },
  { to: '/reports', icon: FileText, label: 'Reports' },
]

const PLATFORM_NAV = [
  { to: '/governance', icon: Shield, label: 'Governance' },
  { to: '/sessions', icon: Video, label: 'Sessions' },
  { to: '/compliance', icon: CheckSquare, label: 'Compliance' },
  { to: '/settings', icon: Settings, label: 'Settings' },
]

export default function Sidebar() {
  const { darkMode, toggleDarkMode } = useAuth()

  return (
    <aside className="w-56 shrink-0 flex flex-col border-r border-slate-200 dark:border-[#272737] bg-white dark:bg-[#11111b] h-screen sticky top-0">
      {/* Brand */}
      <div className="px-5 py-5 border-b border-slate-200 dark:border-[#272737]">
        <div className="flex items-center gap-2">
          <Zap className="w-5 h-5 text-indigo-500" />
          <span className="font-bold text-slate-900 dark:text-slate-100 text-sm tracking-tight">govrix scout</span>
        </div>
        <span className="text-[10px] text-slate-400 font-medium uppercase tracking-widest mt-0.5 block">Open Source</span>
      </div>

      {/* OSS nav */}
      <nav className="flex-1 px-3 py-4 space-y-0.5">
        <p className="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-widest mb-2">Scout</p>
        {OSS_NAV.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-sm font-medium transition-colors ${
                isActive
                  ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-600 dark:text-indigo-400'
                  : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#1a1a2e]'
              }`
            }
          >
            <Icon className="w-4 h-4" />
            {label}
          </NavLink>
        ))}

        {/* Platform nav (locked) */}
        <p className="px-2 text-[10px] font-semibold text-slate-400 uppercase tracking-widest mt-5 mb-2">Platform</p>
        {PLATFORM_NAV.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-sm font-medium transition-colors ${
                isActive
                  ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-600 dark:text-indigo-400'
                  : 'text-slate-500 dark:text-slate-500 hover:bg-slate-100 dark:hover:bg-[#1a1a2e]'
              }`
            }
          >
            <Icon className="w-4 h-4" />
            <span className="flex-1">{label}</span>
            <Lock className="w-3 h-3 text-slate-400" />
          </NavLink>
        ))}
      </nav>

      {/* Footer */}
      <div className="px-4 py-4 border-t border-slate-200 dark:border-[#272737] space-y-2">
        <button
          onClick={toggleDarkMode}
          className="w-full text-left text-xs text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 transition-colors"
        >
          {darkMode ? '☀ Light mode' : '☾ Dark mode'}
        </button>
        <a
          href="https://govrix.io/platform"
          target="_blank"
          rel="noopener noreferrer"
          className="block text-xs text-indigo-500 hover:text-indigo-700 font-medium transition-colors"
        >
          Upgrade to Platform →
        </a>
      </div>
    </aside>
  )
}
```

**Step 2: Header.tsx**

```tsx
// dashboard/src/components/layout/Header.tsx
import { useLocation } from 'react-router-dom'
import { useHealth } from '../../api/hooks'

const PAGE_META: Record<string, { title: string; description: string }> = {
  '/overview': { title: 'Overview', description: 'System health and activity at a glance' },
  '/events': { title: 'Events', description: 'Real-time agent event stream' },
  '/agents': { title: 'Agents', description: 'Registered agents and their activity' },
  '/costs': { title: 'Cost Analytics', description: 'Token usage and spend breakdown' },
  '/reports': { title: 'Reports', description: 'Generate and download audit reports' },
  '/governance': { title: 'Governance', description: 'Risk posture and policy enforcement' },
  '/sessions': { title: 'Session Replay', description: 'Real-time session recording and forensics' },
  '/compliance': { title: 'Compliance', description: 'Framework compliance and audit evidence' },
  '/settings': { title: 'Settings', description: 'System configuration and integrations' },
}

export default function Header() {
  const { pathname } = useLocation()
  const meta = PAGE_META[pathname] ?? { title: 'Govrix Scout', description: '' }
  const { data: health } = useHealth()

  const statusColor =
    health?.status === 'ok' ? 'bg-emerald-500' :
    health?.status === 'degraded' ? 'bg-amber-500' :
    'bg-red-500'

  return (
    <header className="h-14 px-6 flex items-center justify-between border-b border-slate-200 dark:border-[#272737] bg-white dark:bg-[#11111b] sticky top-0 z-10">
      <div>
        <h1 className="text-base font-semibold text-slate-900 dark:text-slate-100">{meta.title}</h1>
        <p className="text-xs text-slate-500">{meta.description}</p>
      </div>
      <div className="flex items-center gap-2">
        <span className={`w-2 h-2 rounded-full ${statusColor}`} />
        <span className="text-xs text-slate-500">
          {health ? `v${health.version}` : 'Connecting...'}
        </span>
      </div>
    </header>
  )
}
```

**Step 3: Layout.tsx**

```tsx
// dashboard/src/components/layout/Layout.tsx
import { Outlet } from 'react-router-dom'
import Sidebar from './Sidebar'
import Header from './Header'

export default function Layout() {
  return (
    <div className="flex min-h-screen">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <Header />
        <main className="flex-1 p-6 bg-slate-50 dark:bg-[#0a0a0f]">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
```

**Step 4: Commit**
```bash
git add dashboard/src/components/layout/
git commit -m "feat(dashboard): add Sidebar, Header, Layout components"
```

---

### Task 7: Common components + EnterpriseGate

**Files:**
- Create: `dashboard/src/components/common/StatusBadge.tsx`
- Create: `dashboard/src/components/common/EmptyState.tsx`
- Create: `dashboard/src/components/common/LoadingState.tsx`
- Create: `dashboard/src/components/EnterpriseGate.tsx`

**Step 1: StatusBadge**

```tsx
// dashboard/src/components/common/StatusBadge.tsx
const COLORS: Record<string, string> = {
  active: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  retired: 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400',
  blocked: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  ok: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  degraded: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  complete: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  pending: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  failed: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
}

export default function StatusBadge({ status }: { status: string }) {
  const cls = COLORS[status] ?? 'bg-slate-100 text-slate-600'
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded-md text-xs font-medium capitalize ${cls}`}>
      {status}
    </span>
  )
}
```

**Step 2: EmptyState**

```tsx
// dashboard/src/components/common/EmptyState.tsx
export default function EmptyState({ message = 'No data yet' }: { message?: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-slate-400">
      <div className="w-10 h-10 rounded-full bg-slate-100 dark:bg-slate-800 flex items-center justify-center mb-3">
        <span className="text-lg">∅</span>
      </div>
      <p className="text-sm">{message}</p>
    </div>
  )
}
```

**Step 3: LoadingState**

```tsx
// dashboard/src/components/common/LoadingState.tsx
export default function LoadingState() {
  return (
    <div className="flex items-center justify-center py-16">
      <div className="w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin" />
    </div>
  )
}
```

**Step 4: EnterpriseGate**

```tsx
// dashboard/src/components/EnterpriseGate.tsx
import { Lock } from 'lucide-react'

interface Props {
  feature: string
  description: string
}

export default function EnterpriseGate({ feature, description }: Props) {
  return (
    <div className="flex-1 flex items-center justify-center p-12">
      <div className="bg-white dark:bg-[#11111b] border border-slate-200 dark:border-[#272737] rounded-2xl p-10 text-center max-w-md shadow-lg">
        <div className="w-14 h-14 rounded-full bg-indigo-50 dark:bg-indigo-950/50 flex items-center justify-center mx-auto mb-5">
          <Lock className="w-7 h-7 text-indigo-400" />
        </div>
        <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100 mb-2">Platform Feature</h2>
        <p className="text-slate-500 text-sm leading-relaxed mb-6">{description}</p>
        <a
          href="https://govrix.io/platform"
          target="_blank"
          rel="noopener noreferrer"
          className="btn-primary inline-block w-full text-center py-3 rounded-xl"
        >
          Learn about Platform
        </a>
        <p className="text-xs text-slate-400 mt-3">Free for open-source projects</p>
      </div>
    </div>
  )
}
```

**Step 5: Commit**
```bash
git add dashboard/src/components/
git commit -m "feat(dashboard): add common components and EnterpriseGate"
```

---

### Task 8: App router

**Files:**
- Create: `dashboard/src/App.tsx`

**Step 1: Write App.tsx with all routes**

```tsx
// dashboard/src/App.tsx
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { AuthProvider } from './context/AuthContext'
import Layout from './components/layout/Layout'
import OverviewPage from './pages/OverviewPage'
import EventsPage from './pages/EventsPage'
import AgentsPage from './pages/AgentsPage'
import CostsPage from './pages/CostsPage'
import ReportsPage from './pages/ReportsPage'
import EnterpriseGate from './components/EnterpriseGate'

const GATED = [
  {
    path: 'governance',
    feature: 'Governance',
    description: 'Upgrade to Govrix Platform to unlock risk posture dashboards, policy enforcement, kill switches, and PII detection activity.',
  },
  {
    path: 'sessions',
    feature: 'Session Replay',
    description: 'Upgrade to Govrix Platform to unlock real-time session recording, forensic replay, and compliance evidence capture.',
  },
  {
    path: 'compliance',
    feature: 'Compliance',
    description: 'Upgrade to Govrix Platform to unlock automated compliance reporting for SOC 2, EU AI Act, HIPAA, and NIST.',
  },
  {
    path: 'settings',
    feature: 'Settings',
    description: 'Upgrade to Govrix Platform to manage system configuration, SIEM connectors, mTLS certificates, and multi-tenancy.',
  },
]

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Navigate to="/overview" replace />} />
            <Route path="overview" element={<OverviewPage />} />
            <Route path="events" element={<EventsPage />} />
            <Route path="agents" element={<AgentsPage />} />
            <Route path="costs" element={<CostsPage />} />
            <Route path="reports" element={<ReportsPage />} />
            {GATED.map(({ path, description }) => (
              <Route
                key={path}
                path={path}
                element={<EnterpriseGate feature={path} description={description} />}
              />
            ))}
            <Route path="*" element={<Navigate to="/overview" replace />} />
          </Route>
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  )
}
```

**Step 2: Commit**
```bash
git add dashboard/src/App.tsx
git commit -m "feat(dashboard): add router with OSS routes + gated platform routes"
```

---

### Task 9: OverviewPage

**Files:**
- Create: `dashboard/src/pages/OverviewPage.tsx`

**Step 1: Write OverviewPage wired to health + cost summary + agents**

```tsx
// dashboard/src/pages/OverviewPage.tsx
import { useHealth, useCostSummary, useAgents, useEvents } from '../api/hooks'
import LoadingState from '../components/common/LoadingState'
import { Activity, Users, DollarSign, Zap } from 'lucide-react'

function StatCard({ label, value, icon: Icon, sub }: {
  label: string; value: string; icon: React.ElementType; sub?: string
}) {
  return (
    <div className="card">
      <div className="flex items-center justify-between mb-3">
        <span className="text-sm font-medium text-slate-500">{label}</span>
        <Icon className="w-4 h-4 text-slate-400" />
      </div>
      <p className="text-2xl font-bold text-slate-900 dark:text-slate-100">{value}</p>
      {sub && <p className="text-xs text-slate-500 mt-1">{sub}</p>}
    </div>
  )
}

export default function OverviewPage() {
  const { data: health, isLoading: hLoading } = useHealth()
  const { data: costs, isLoading: cLoading } = useCostSummary()
  const { data: agents } = useAgents()
  const { data: events } = useEvents({ limit: 10 })

  if (hLoading || cLoading) return <LoadingState />

  const activeAgents = agents?.data.filter(a => a.status === 'active').length ?? 0
  const totalCost = costs?.total_cost_usd?.toFixed(4) ?? '0.0000'
  const totalRequests = costs?.total_requests?.toLocaleString() ?? '0'

  return (
    <div className="space-y-6">
      {/* KPI row */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard label="Active Agents" value={String(activeAgents)} icon={Users} sub={`${agents?.total ?? 0} total registered`} />
        <StatCard label="Total Requests" value={totalRequests} icon={Activity} sub="all time" />
        <StatCard label="Total Cost" value={`$${totalCost}`} icon={DollarSign} sub="USD, all time" />
        <StatCard label="Backend" value={health?.status ?? '—'} icon={Zap} sub={`v${health?.version ?? '?'}`} />
      </div>

      {/* Recent events */}
      <div className="card">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Recent Events</h2>
        {events?.data.length === 0 ? (
          <p className="text-sm text-slate-400 text-center py-8">No events yet — traffic will appear here</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                <th className="pb-2 font-medium">Agent</th>
                <th className="pb-2 font-medium">Kind</th>
                <th className="pb-2 font-medium">Model</th>
                <th className="pb-2 font-medium">Cost</th>
                <th className="pb-2 font-medium">Time</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/50">
              {events?.data.slice(0, 10).map(e => (
                <tr key={e.id} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2 text-slate-700 dark:text-slate-300 font-mono text-xs truncate max-w-[120px]">{e.agent_id}</td>
                  <td className="py-2 text-slate-500">{e.kind}</td>
                  <td className="py-2 text-slate-500">{e.model ?? '—'}</td>
                  <td className="py-2 text-slate-500">{e.cost_usd != null ? `$${e.cost_usd.toFixed(5)}` : '—'}</td>
                  <td className="py-2 text-slate-400 text-xs">{new Date(e.timestamp).toLocaleTimeString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
```

**Step 2: Commit**
```bash
git add dashboard/src/pages/OverviewPage.tsx
git commit -m "feat(dashboard): add OverviewPage wired to health, costs, agents, events"
```

---

### Task 10: EventsPage

**Files:**
- Create: `dashboard/src/pages/EventsPage.tsx`

**Step 1: Write EventsPage with pagination and expandable rows**

```tsx
// dashboard/src/pages/EventsPage.tsx
import { useState } from 'react'
import { useEvents } from '../api/hooks'
import StatusBadge from '../components/common/StatusBadge'
import LoadingState from '../components/common/LoadingState'
import EmptyState from '../components/common/EmptyState'
import type { AgentEvent } from '../api/types'

const PAGE_SIZE = 25

function EventRow({ event }: { event: AgentEvent }) {
  const [expanded, setExpanded] = useState(false)
  return (
    <>
      <tr
        className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50 cursor-pointer"
        onClick={() => setExpanded(e => !e)}
      >
        <td className="py-2.5 px-3 font-mono text-xs text-slate-500">{event.id.slice(0, 8)}</td>
        <td className="py-2.5 px-3 text-xs text-slate-600 dark:text-slate-400">{event.kind}</td>
        <td className="py-2.5 px-3 text-xs text-slate-500 font-mono truncate max-w-[120px]">{event.agent_id.slice(0, 12)}</td>
        <td className="py-2.5 px-3 text-xs">{event.model ?? '—'}</td>
        <td className="py-2.5 px-3 text-xs">{event.cost_usd != null ? `$${event.cost_usd.toFixed(5)}` : '—'}</td>
        <td className="py-2.5 px-3"><StatusBadge status={event.pii_detected ? 'blocked' : 'ok'} /></td>
        <td className="py-2.5 px-3 text-xs text-slate-400">{new Date(event.timestamp).toLocaleString()}</td>
      </tr>
      {expanded && (
        <tr className="bg-slate-50 dark:bg-[#0d0d1a]">
          <td colSpan={7} className="px-4 py-3">
            <div className="grid grid-cols-2 gap-4 text-xs">
              <div>
                <p className="text-slate-400 mb-1">Session ID</p>
                <p className="font-mono text-slate-700 dark:text-slate-300">{event.session_id}</p>
              </div>
              <div>
                <p className="text-slate-400 mb-1">Lineage Hash</p>
                <p className="font-mono text-slate-700 dark:text-slate-300">{event.lineage_hash}</p>
              </div>
              <div>
                <p className="text-slate-400 mb-1">Compliance Tag</p>
                <p className="font-mono text-slate-700 dark:text-slate-300">{event.compliance_tag}</p>
              </div>
              <div>
                <p className="text-slate-400 mb-1">Latency</p>
                <p className="text-slate-700 dark:text-slate-300">{event.latency_ms != null ? `${event.latency_ms}ms` : '—'}</p>
              </div>
            </div>
          </td>
        </tr>
      )}
    </>
  )
}

export default function EventsPage() {
  const [page, setPage] = useState(0)
  const { data, isLoading } = useEvents({ limit: PAGE_SIZE, offset: page * PAGE_SIZE })

  if (isLoading) return <LoadingState />

  const total = data?.total ?? 0
  const pages = Math.ceil(total / PAGE_SIZE)

  return (
    <div className="card overflow-hidden">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
          Event Stream <span className="text-slate-400 font-normal ml-1">({total.toLocaleString()} total)</span>
        </h2>
      </div>
      {data?.data.length === 0 ? <EmptyState message="No events yet" /> : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                  {['ID', 'Kind', 'Agent', 'Model', 'Cost', 'PII', 'Time'].map(h => (
                    <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
                {data?.data.map(e => <EventRow key={e.id} event={e} />)}
              </tbody>
            </table>
          </div>
          {pages > 1 && (
            <div className="flex items-center justify-between mt-4 pt-4 border-t border-slate-100 dark:border-slate-800">
              <button
                onClick={() => setPage(p => Math.max(0, p - 1))}
                disabled={page === 0}
                className="text-sm text-slate-500 disabled:opacity-40 hover:text-slate-700"
              >← Prev</button>
              <span className="text-xs text-slate-400">{page + 1} / {pages}</span>
              <button
                onClick={() => setPage(p => Math.min(pages - 1, p + 1))}
                disabled={page >= pages - 1}
                className="text-sm text-slate-500 disabled:opacity-40 hover:text-slate-700"
              >Next →</button>
            </div>
          )}
        </>
      )}
    </div>
  )
}
```

**Step 2: Commit**
```bash
git add dashboard/src/pages/EventsPage.tsx
git commit -m "feat(dashboard): add EventsPage with pagination and expandable compliance fields"
```

---

### Task 11: AgentsPage

**Files:**
- Create: `dashboard/src/pages/AgentsPage.tsx`

**Step 1: Write AgentsPage wired to agents API with retire action**

```tsx
// dashboard/src/pages/AgentsPage.tsx
import { useAgents, useRetireAgent } from '../api/hooks'
import StatusBadge from '../components/common/StatusBadge'
import LoadingState from '../components/common/LoadingState'
import EmptyState from '../components/common/EmptyState'

export default function AgentsPage() {
  const { data, isLoading } = useAgents()
  const retire = useRetireAgent()

  if (isLoading) return <LoadingState />

  return (
    <div className="card overflow-hidden">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
          Agent Registry <span className="text-slate-400 font-normal ml-1">({data?.total ?? 0} agents)</span>
        </h2>
      </div>
      {data?.data.length === 0 ? (
        <EmptyState message="No agents registered yet — agents appear automatically on first request" />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['Name / ID', 'Status', 'Requests', 'Cost (USD)', 'Tokens', 'Last Seen', 'Actions'].map(h => (
                  <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
              {data?.data.map(agent => (
                <tr key={agent.id} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-3 px-3">
                    <p className="font-medium text-slate-900 dark:text-slate-100">{agent.name}</p>
                    <p className="text-xs text-slate-400 font-mono">{agent.id.slice(0, 12)}</p>
                  </td>
                  <td className="py-3 px-3"><StatusBadge status={agent.status} /></td>
                  <td className="py-3 px-3 text-slate-600 dark:text-slate-400">{agent.total_requests.toLocaleString()}</td>
                  <td className="py-3 px-3 text-slate-600 dark:text-slate-400">${agent.total_cost_usd.toFixed(4)}</td>
                  <td className="py-3 px-3 text-slate-500 text-xs">
                    {(agent.total_input_tokens + agent.total_output_tokens).toLocaleString()}
                  </td>
                  <td className="py-3 px-3 text-xs text-slate-400">
                    {new Date(agent.last_seen).toLocaleDateString()}
                  </td>
                  <td className="py-3 px-3">
                    {agent.status === 'active' && (
                      <button
                        onClick={() => retire.mutate(agent.id)}
                        disabled={retire.isPending}
                        className="text-xs text-red-500 hover:text-red-700 disabled:opacity-40"
                      >
                        Retire
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
```

**Step 2: Commit**
```bash
git add dashboard/src/pages/AgentsPage.tsx
git commit -m "feat(dashboard): add AgentsPage with retire action"
```

---

### Task 12: CostsPage + ReportsPage

**Files:**
- Create: `dashboard/src/pages/CostsPage.tsx`
- Create: `dashboard/src/pages/ReportsPage.tsx`

**Step 1: CostsPage**

```tsx
// dashboard/src/pages/CostsPage.tsx
import { useCostSummary, useCostBreakdown } from '../api/hooks'
import LoadingState from '../components/common/LoadingState'

export default function CostsPage() {
  const { data: summary, isLoading } = useCostSummary()
  const { data: breakdown } = useCostBreakdown()

  if (isLoading) return <LoadingState />

  return (
    <div className="space-y-6">
      {/* Summary KPIs */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {[
          { label: 'Total Cost', value: `$${summary?.total_cost_usd?.toFixed(4) ?? '0'}` },
          { label: 'Total Requests', value: summary?.total_requests?.toLocaleString() ?? '0' },
          { label: 'Avg Cost / Req', value: `$${summary?.avg_cost_per_request?.toFixed(5) ?? '0'}` },
          { label: 'Total Tokens', value: ((summary?.total_input_tokens ?? 0) + (summary?.total_output_tokens ?? 0)).toLocaleString() },
        ].map(({ label, value }) => (
          <div key={label} className="card">
            <p className="text-xs text-slate-500 mb-1">{label}</p>
            <p className="text-xl font-bold text-slate-900 dark:text-slate-100">{value}</p>
          </div>
        ))}
      </div>

      {/* By Model */}
      {breakdown?.by_model && (
        <div className="card">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Breakdown by Model</h2>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['Model', 'Requests', 'Cost (USD)', 'Input Tokens', 'Output Tokens'].map(h => (
                  <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
              {breakdown.by_model.map(row => (
                <tr key={row.label} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2.5 px-3 font-medium text-slate-900 dark:text-slate-100">{row.label}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.requests.toLocaleString()}</td>
                  <td className="py-2.5 px-3 text-slate-500">${row.cost_usd.toFixed(4)}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.input_tokens.toLocaleString()}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.output_tokens.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* By Agent */}
      {breakdown?.by_agent && breakdown.by_agent.length > 0 && (
        <div className="card">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Breakdown by Agent</h2>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['Agent', 'Requests', 'Cost (USD)'].map(h => (
                  <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
              {breakdown.by_agent.map(row => (
                <tr key={row.label} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2.5 px-3 font-mono text-xs text-slate-700 dark:text-slate-300">{row.label}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.requests.toLocaleString()}</td>
                  <td className="py-2.5 px-3 text-slate-500">${row.cost_usd.toFixed(4)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
```

**Step 2: ReportsPage**

```tsx
// dashboard/src/pages/ReportsPage.tsx
import { useState } from 'react'
import { useReports, useReportTypes, useGenerateReport } from '../api/hooks'
import StatusBadge from '../components/common/StatusBadge'
import LoadingState from '../components/common/LoadingState'

export default function ReportsPage() {
  const { data: types } = useReportTypes()
  const { data: reports, isLoading } = useReports()
  const generate = useGenerateReport()
  const [selectedType, setSelectedType] = useState('')
  const [format, setFormat] = useState<'pdf' | 'json' | 'csv'>('pdf')

  if (isLoading) return <LoadingState />

  return (
    <div className="space-y-6">
      {/* Generate */}
      <div className="card">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Generate Report</h2>
        <div className="flex gap-3 flex-wrap">
          <select
            value={selectedType}
            onChange={e => setSelectedType(e.target.value)}
            className="flex-1 min-w-[180px] text-sm bg-white dark:bg-[#0d0d1a] border border-slate-200 dark:border-[#272737] rounded-lg px-3 py-2 text-slate-700 dark:text-slate-300"
          >
            <option value="">Select report type…</option>
            {types?.data.map(t => <option key={t.id} value={t.id}>{t.name}</option>)}
          </select>
          <select
            value={format}
            onChange={e => setFormat(e.target.value as typeof format)}
            className="text-sm bg-white dark:bg-[#0d0d1a] border border-slate-200 dark:border-[#272737] rounded-lg px-3 py-2 text-slate-700 dark:text-slate-300"
          >
            <option value="pdf">PDF</option>
            <option value="json">JSON</option>
            <option value="csv">CSV</option>
          </select>
          <button
            onClick={() => generate.mutate({ report_type: selectedType, format })}
            disabled={!selectedType || generate.isPending}
            className="btn-primary disabled:opacity-40"
          >
            {generate.isPending ? 'Generating…' : 'Generate'}
          </button>
        </div>
      </div>

      {/* Report history */}
      <div className="card">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">
          Generated Reports <span className="text-slate-400 font-normal ml-1">({reports?.total ?? 0})</span>
        </h2>
        {reports?.data.length === 0 ? (
          <p className="text-sm text-slate-400 text-center py-8">No reports generated yet</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['ID', 'Type', 'Status', 'Created', 'Download'].map(h => (
                  <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
              {reports?.data.map(r => (
                <tr key={r.id} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2.5 px-3 font-mono text-xs text-slate-500">{r.id.slice(0, 8)}</td>
                  <td className="py-2.5 px-3 text-slate-600 dark:text-slate-400">{r.report_type}</td>
                  <td className="py-2.5 px-3"><StatusBadge status={r.status} /></td>
                  <td className="py-2.5 px-3 text-xs text-slate-400">{new Date(r.created_at).toLocaleString()}</td>
                  <td className="py-2.5 px-3">
                    {r.download_url ? (
                      <a href={r.download_url} className="text-xs text-indigo-500 hover:underline">Download</a>
                    ) : <span className="text-xs text-slate-400">—</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
```

**Step 3: Commit**
```bash
git add dashboard/src/pages/CostsPage.tsx dashboard/src/pages/ReportsPage.tsx
git commit -m "feat(dashboard): add CostsPage and ReportsPage wired to backend"
```

---

### Task 13: Verify and push to main

**Step 1: Run dev server and verify all pages load**
```bash
cd /Users/manas.choudhary/Documents/Project/govrix/govrix-scout/dashboard
pnpm dev
```
Open http://localhost:3000 — verify:
- [ ] Overview loads with KPI cards
- [ ] Events loads with paginated table
- [ ] Agents loads with retire button
- [ ] Costs loads with summary + breakdown
- [ ] Reports loads with generate form
- [ ] Governance/Sessions/Compliance/Settings show EnterpriseGate card

**Step 2: Check for TypeScript errors**
```bash
cd /Users/manas.choudhary/Documents/Project/govrix/govrix-scout/dashboard
pnpm tsc --noEmit
```
Expected: 0 errors

**Step 3: Final commit and push**
```bash
cd /Users/manas.choudhary/Documents/Project/govrix/govrix-scout
git add -A
git commit -m "feat(dashboard): complete TypeScript dashboard rewrite — OSS pages wired, platform gated"
git push origin main
```
