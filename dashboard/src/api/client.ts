// dashboard/src/api/client.ts
import type {
  AgentEvent, Agent, AgentRun, CostSummary, CostBreakdown, CostBucket, CostTimeseriesBucket,
  HealthResponse, Report, ReportType, GenerateReportRequest,
  SystemConfig, PaginatedResponse, EventFilters,
  AgentBudgetResponse, BudgetOverviewRow, SetBudgetRequest,
  Project, CreateProjectRequest, ProjectCostSummary,
} from './types'

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message)
    this.name = 'ApiError'
  }
}

const API_KEY = (import.meta as unknown as { env: Record<string, string> }).env?.VITE_API_KEY ?? 'govrix-local-dev'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
      ...init?.headers,
    },
    ...init,
  })
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText)
    throw new ApiError(res.status, text)
  }
  return res.json() as Promise<T>
}

type QueryParams = Record<string, string | number | boolean | undefined | null>

function buildParams(filters: QueryParams): string {
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
  request<PaginatedResponse<AgentEvent>>(`/api/v1/events${buildParams(filters as QueryParams)}`)
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
  request<PaginatedResponse<AgentEvent>>(`/api/v1/agents/${id}/events${buildParams(filters as QueryParams)}`)
export const getAgentRuns = (id: string, limit = 50) =>
  request<PaginatedResponse<AgentRun>>(`/api/v1/agents/${id}/runs?limit=${limit}`)
export const getAgentViolations = (id: string, limit = 50) =>
  request<PaginatedResponse<AgentEvent>>(`/api/v1/agents/${id}/violations?limit=${limit}`)

// Costs — unwrap {"data": ...} envelope from API responses

interface RawCostSummary {
  from: string; to: string
  total_requests: number; total_input_tokens: number; total_output_tokens: number
  total_cost_usd: number; avg_latency_ms: number | null; p99_latency_ms: number | null
}

interface RawBreakdownRow {
  group_key: string; request_count: number
  total_input_tokens: number; total_output_tokens: number
  total_cost_usd: number; avg_latency_ms: number | null
}

export const getCostSummary = async (): Promise<CostSummary> => {
  const resp = await request<{ data: RawCostSummary }>('/api/v1/costs/summary')
  const d = resp.data
  return {
    total_cost_usd: d.total_cost_usd ?? 0,
    total_requests: d.total_requests ?? 0,
    total_input_tokens: d.total_input_tokens ?? 0,
    total_output_tokens: d.total_output_tokens ?? 0,
    avg_cost_per_request: d.total_requests > 0 ? d.total_cost_usd / d.total_requests : 0,
    period_start: d.from,
    period_end: d.to,
  }
}

const mapBreakdownRow = (r: RawBreakdownRow): CostBucket => ({
  label: r.group_key,
  cost_usd: r.total_cost_usd,
  requests: r.request_count,
  input_tokens: r.total_input_tokens,
  output_tokens: r.total_output_tokens,
})

export const getCostBreakdown = async (): Promise<CostBreakdown> => {
  const [modelResp, agentResp] = await Promise.all([
    request<{ data: RawBreakdownRow[] }>('/api/v1/costs/breakdown?group_by=model'),
    request<{ data: RawBreakdownRow[] }>('/api/v1/costs/breakdown?group_by=agent'),
  ])
  return {
    by_model: modelResp.data.map(mapBreakdownRow),
    by_agent: agentResp.data.map(mapBreakdownRow),
    by_provider: [],
  }
}
export const getCostTimeseries = (params: { agent_id?: string; from?: string; to?: string; granularity?: string } = {}) =>
  request<PaginatedResponse<CostTimeseriesBucket>>(`/api/v1/costs/timeseries${buildParams(params as QueryParams)}`)

// Reports
export const getReportTypes = () =>
  request<PaginatedResponse<ReportType>>('/api/v1/reports/types')
export const getReports = () =>
  request<PaginatedResponse<Report>>('/api/v1/reports')
export const generateReport = (body: GenerateReportRequest) =>
  request<Report>('/api/v1/reports/generate', { method: 'POST', body: JSON.stringify(body) })

// Budgets
export const getAgentBudget = (id: string) =>
  request<{ data: AgentBudgetResponse }>(`/api/v1/agents/${id}/budget`)
export const setAgentBudget = (id: string, body: SetBudgetRequest) =>
  request<{ data: unknown }>(`/api/v1/agents/${id}/budget`, { method: 'PUT', body: JSON.stringify(body) })
export const deleteAgentBudget = (id: string) =>
  request<{ data: unknown }>(`/api/v1/agents/${id}/budget`, { method: 'DELETE' })
export const getBudgetOverview = () =>
  request<PaginatedResponse<BudgetOverviewRow>>('/api/v1/budgets/overview')

// Projects
export const getProjects = () =>
  request<PaginatedResponse<Project>>('/api/v1/projects')
export const createProject = (body: CreateProjectRequest) =>
  request<{ data: Project }>('/api/v1/projects', { method: 'POST', body: JSON.stringify(body) })
export const getProject = (id: string) =>
  request<{ data: Project }>(`/api/v1/projects/${id}`)
export const updateProject = (id: string, body: Partial<CreateProjectRequest>) =>
  request<{ data: Project }>(`/api/v1/projects/${id}`, { method: 'PUT', body: JSON.stringify(body) })
export const deleteProject = (id: string) =>
  request<{ data: unknown }>(`/api/v1/projects/${id}`, { method: 'DELETE' })
export const getProjectAgents = (id: string) =>
  request<PaginatedResponse<Agent>>(`/api/v1/projects/${id}/agents`)
export const getProjectCosts = (id: string) =>
  request<{ data: ProjectCostSummary }>(`/api/v1/projects/${id}/costs`)
export const assignAgentProject = (agentId: string, projectId: string | null) =>
  request<{ data: unknown }>(`/api/v1/agents/${agentId}/project`, { method: 'PUT', body: JSON.stringify({ project_id: projectId }) })

// Config
export const getConfig = () =>
  request<SystemConfig>('/api/v1/config')
