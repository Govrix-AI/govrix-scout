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
  agentRuns: (id: string) => ['agents', id, 'runs'] as const,
  agentViolations: (id: string) => ['agents', id, 'violations'] as const,
  costSummary: ['costs', 'summary'] as const,
  costBreakdown: ['costs', 'breakdown'] as const,
  costTimeseries: (params: Record<string, string | undefined>) => ['costs', 'timeseries', params] as const,
  reportTypes: ['reports', 'types'] as const,
  reports: ['reports'] as const,
  agentBudget: (id: string) => ['agents', id, 'budget'] as const,
  budgetOverview: ['budgets', 'overview'] as const,
  projects: ['projects'] as const,
  project: (id: string) => ['projects', id] as const,
  projectAgents: (id: string) => ['projects', id, 'agents'] as const,
  projectCosts: (id: string) => ['projects', id, 'costs'] as const,
  config: ['config'] as const,
}

export const useHealth = () =>
  useQuery({ queryKey: qk.health, queryFn: api.getHealth, refetchInterval: 10_000 })

export const useEvents = (filters: EventFilters = {}) =>
  useQuery({ queryKey: qk.events(filters), queryFn: () => api.getEvents(filters), refetchInterval: 5_000 })

export const useEvent = (id: string) =>
  useQuery({ queryKey: qk.event(id), queryFn: () => api.getEvent(id), enabled: !!id })

export const useSessionEvents = (sessionId: string) =>
  useQuery({ queryKey: qk.sessionEvents(sessionId), queryFn: () => api.getSessionEvents(sessionId), enabled: !!sessionId })

export const useAgents = () =>
  useQuery({ queryKey: qk.agents, queryFn: api.getAgents, refetchInterval: 10_000 })

export const useAgent = (id: string) =>
  useQuery({ queryKey: qk.agent(id), queryFn: () => api.getAgent(id), enabled: !!id })

export const useAgentEvents = (id: string, filters: EventFilters = {}) =>
  useQuery({ queryKey: qk.agentEvents(id, filters), queryFn: () => api.getAgentEvents(id, filters), enabled: !!id })

export const useAgentRuns = (id: string) =>
  useQuery({ queryKey: qk.agentRuns(id), queryFn: () => api.getAgentRuns(id), enabled: !!id, refetchInterval: 10_000 })

export const useAgentViolations = (id: string) =>
  useQuery({ queryKey: qk.agentViolations(id), queryFn: () => api.getAgentViolations(id), enabled: !!id, refetchInterval: 10_000 })

export const useCostTimeseries = (params: { agent_id?: string; from?: string; to?: string; granularity?: string } = {}) =>
  useQuery({ queryKey: qk.costTimeseries(params), queryFn: () => api.getCostTimeseries(params), refetchInterval: 30_000 })

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

export const useCostSummary = () =>
  useQuery({ queryKey: qk.costSummary, queryFn: api.getCostSummary, refetchInterval: 30_000 })

export const useCostBreakdown = () =>
  useQuery({ queryKey: qk.costBreakdown, queryFn: api.getCostBreakdown, refetchInterval: 30_000 })

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

export const useConfig = () =>
  useQuery({ queryKey: qk.config, queryFn: api.getConfig, staleTime: 60_000 })

// ── Budgets ───────────────────────────────────────────────────────────────────

export const useAgentBudget = (id: string) =>
  useQuery({ queryKey: qk.agentBudget(id), queryFn: () => api.getAgentBudget(id), enabled: !!id, refetchInterval: 10_000 })

export const useBudgetOverview = () =>
  useQuery({ queryKey: qk.budgetOverview, queryFn: api.getBudgetOverview, refetchInterval: 15_000 })

export const useSetAgentBudget = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: Parameters<typeof api.setAgentBudget>[1] }) =>
      api.setAgentBudget(id, body),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: qk.agentBudget(vars.id) })
      qc.invalidateQueries({ queryKey: qk.budgetOverview })
    },
  })
}

export const useDeleteAgentBudget = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.deleteAgentBudget(id),
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: qk.agentBudget(id) })
      qc.invalidateQueries({ queryKey: qk.budgetOverview })
    },
  })
}

// ── Projects ──────────────────────────────────────────────────────────────────

export const useProjects = () =>
  useQuery({ queryKey: qk.projects, queryFn: api.getProjects, refetchInterval: 15_000 })

export const useProject = (id: string) =>
  useQuery({ queryKey: qk.project(id), queryFn: () => api.getProject(id), enabled: !!id })

export const useProjectAgents = (id: string) =>
  useQuery({ queryKey: qk.projectAgents(id), queryFn: () => api.getProjectAgents(id), enabled: !!id, refetchInterval: 10_000 })

export const useProjectCosts = (id: string) =>
  useQuery({ queryKey: qk.projectCosts(id), queryFn: () => api.getProjectCosts(id), enabled: !!id })

export const useCreateProject = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: Parameters<typeof api.createProject>[0]) => api.createProject(body),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.projects }) },
  })
}

export const useUpdateProject = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: Parameters<typeof api.updateProject>[1] }) =>
      api.updateProject(id, body),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.projects }) },
  })
}

export const useDeleteProject = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.deleteProject(id),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.projects }) },
  })
}

export const useAssignAgentProject = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ agentId, projectId }: { agentId: string; projectId: string | null }) =>
      api.assignAgentProject(agentId, projectId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.projects })
      qc.invalidateQueries({ queryKey: qk.agents })
    },
  })
}
