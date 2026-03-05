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
  name: string | null
  description: string | null
  agent_type: string | null
  status: 'active' | 'idle' | 'error' | 'retired' | 'blocked'
  first_seen_at: string
  last_seen_at: string
  last_error_at: string | null
  source_ip: string | null
  fingerprint: string | null
  target_apis: unknown | null
  mcp_servers: unknown | null
  total_requests: number
  total_tokens_in: number
  total_tokens_out: number
  total_cost_usd: number
  last_model_used: string | null
  error_count: number
  labels: Record<string, unknown> | null
  metadata: Record<string, unknown> | null
  created_at: string
  updated_at: string
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

export interface AgentRun {
  session_id: string
  started_at: string
  ended_at: string
  event_count: number
  model: string | null
  total_input_tokens: number
  total_output_tokens: number
  total_cost_usd: number
  max_latency_ms: number | null
  status_code: number | null
  has_violations: boolean
}

export interface CostTimeseriesBucket {
  bucket: string
  request_count: number
  total_cost_usd: number
  total_tokens: number
}

// ── Budget types ──────────────────────────────────────────────────────────────

export interface AgentBudgetResponse {
  agent_id: string
  config: {
    agent_id: string
    daily_token_limit: number | null
    daily_cost_limit_usd: number | null
    monthly_cost_limit_usd: number | null
    created_at: string
    updated_at: string
  } | null
  usage: {
    tokens_used_today: number
    cost_used_today: number
  }
}

export interface BudgetOverviewRow {
  agent_id: string
  name: string | null
  status: string
  daily_token_limit: number | null
  daily_cost_limit_usd: number | null
  monthly_cost_limit_usd: number | null
  tokens_used_today: number
  cost_used_today: number
}

export interface SetBudgetRequest {
  daily_token_limit?: number | null
  daily_cost_limit_usd?: number | null
  monthly_cost_limit_usd?: number | null
}

// ── Project types ─────────────────────────────────────────────────────────────

export interface Project {
  id: string
  name: string
  description: string | null
  agent_count: number
  total_cost_usd: number
  created_at: string
  updated_at: string
}

export interface CreateProjectRequest {
  name: string
  description?: string
}

export interface ProjectCostSummary {
  project_id: string
  from: string
  to: string
  total_requests: number
  total_input_tokens: number
  total_output_tokens: number
  total_cost_usd: number
}
