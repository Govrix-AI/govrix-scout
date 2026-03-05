import { useState } from 'react'
import { useParams, Link, useNavigate } from 'react-router-dom'
import { ArrowLeft, FolderOpen, DollarSign, Bot, Trash2 } from 'lucide-react'
import { useProject, useProjectAgents, useProjectCosts, useAssignAgentProject, useDeleteProject, useAgents } from '../api/hooks'

const fmtNum = (n: number | undefined | null): string =>
  typeof n === 'number' ? n.toLocaleString() : '0'

const fmtUsd = (n: number | undefined | null): string =>
  typeof n === 'number' ? `$${n.toFixed(4)}` : '$0'

const fmtDate = (s: string | undefined | null): string => {
  if (!s) return '\u2014'
  return new Date(s).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { data: projectData, isLoading } = useProject(id ?? '')
  const { data: agentsData } = useProjectAgents(id ?? '')
  const { data: costsData } = useProjectCosts(id ?? '')
  const { data: allAgentsData } = useAgents()
  const assignAgent = useAssignAgentProject()
  const deleteProject = useDeleteProject()

  const [showAddAgent, setShowAddAgent] = useState(false)

  const project = (projectData as any)?.data ?? projectData
  const agents = agentsData?.data ?? []
  const costs = (costsData as any)?.data ?? costsData
  const allAgents = allAgentsData?.data ?? []
  const unassignedAgents = allAgents.filter((a: any) => !agents.some((pa: any) => pa.id === a.id))

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="animate-pulse text-slate-400 text-sm">Loading project...</div>
      </div>
    )
  }

  if (!project) {
    return (
      <div className="flex-1 flex items-center justify-center flex-col gap-4">
        <FolderOpen className="w-12 h-12 text-slate-300" />
        <p className="text-sm text-slate-500">Project not found</p>
        <Link to="/projects" className="text-xs text-primary hover:underline">Back to projects</Link>
      </div>
    )
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-4">

        <Link to="/projects" className="inline-flex items-center gap-1.5 text-xs text-slate-400 hover:text-primary transition-colors mb-2">
          <ArrowLeft className="w-3.5 h-3.5" />
          Back to Projects
        </Link>

        {/* Header */}
        <div className="bg-white border border-slate-200 rounded-xl p-5">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-lg font-bold text-slate-900">{project.name}</h2>
              {project.description && <p className="text-xs text-slate-400 mt-1">{project.description}</p>}
            </div>
            <button
              onClick={async () => {
                if (confirm('Delete this project?')) {
                  await deleteProject.mutateAsync(id!)
                  navigate('/projects')
                }
              }}
              className="text-xs text-red-400 hover:text-red-600 flex items-center gap-1"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Delete
            </button>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-3 gap-4 mt-5">
            <div className="bg-slate-50 rounded-lg p-3">
              <div className="flex items-center gap-1.5 mb-1">
                <Bot className="w-3.5 h-3.5 text-primary" />
                <span className="text-[10px] uppercase tracking-wider text-slate-400 font-bold">Agents</span>
              </div>
              <p className="text-lg font-bold metric-font text-slate-800">{project.agent_count ?? agents.length}</p>
            </div>
            <div className="bg-slate-50 rounded-lg p-3">
              <div className="flex items-center gap-1.5 mb-1">
                <DollarSign className="w-3.5 h-3.5 text-emerald-500" />
                <span className="text-[10px] uppercase tracking-wider text-slate-400 font-bold">Total Cost</span>
              </div>
              <p className="text-lg font-bold metric-font text-slate-800">{fmtUsd(costs?.total_cost_usd ?? project.total_cost_usd)}</p>
            </div>
            <div className="bg-slate-50 rounded-lg p-3">
              <div className="flex items-center gap-1.5 mb-1">
                <span className="text-[10px] uppercase tracking-wider text-slate-400 font-bold">Requests (30d)</span>
              </div>
              <p className="text-lg font-bold metric-font text-slate-800">{fmtNum(costs?.total_requests)}</p>
            </div>
          </div>
        </div>

        {/* Agents Table */}
        <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
          <div className="p-4 border-b border-slate-100 flex items-center justify-between">
            <div>
              <h3 className="text-sm font-bold text-slate-700">Agents</h3>
              <p className="text-xs text-slate-400">{agents.length} agent{agents.length !== 1 ? 's' : ''} in this project</p>
            </div>
            <button
              onClick={() => setShowAddAgent(!showAddAgent)}
              className="btn-secondary text-xs"
            >
              Add Agent
            </button>
          </div>

          {showAddAgent && unassignedAgents.length > 0 && (
            <div className="p-4 bg-slate-50 border-b border-slate-100">
              <p className="text-xs text-slate-500 mb-2">Select an agent to add:</p>
              <div className="flex flex-wrap gap-2">
                {unassignedAgents.slice(0, 20).map((a: any) => (
                  <button
                    key={a.id}
                    onClick={async () => {
                      await assignAgent.mutateAsync({ agentId: a.id, projectId: id! })
                      setShowAddAgent(false)
                    }}
                    className="text-xs bg-white border border-slate-200 rounded-lg px-3 py-1.5 hover:border-primary/30 hover:text-primary transition-colors"
                  >
                    {a.name || a.id?.slice(0, 20)}
                  </button>
                ))}
              </div>
            </div>
          )}

          <table className="w-full">
            <thead>
              <tr className="border-b border-slate-100 bg-slate-50/50">
                <th className="table-header text-left py-3 px-4">Agent</th>
                <th className="table-header text-center py-3 px-4">Status</th>
                <th className="table-header text-right py-3 px-4">Requests</th>
                <th className="table-header text-right py-3 px-4">Cost</th>
                <th className="table-header text-left py-3 px-4">Last Seen</th>
                <th className="table-header text-center py-3 px-4">Actions</th>
              </tr>
            </thead>
            <tbody>
              {agents.map((a: any, i: number) => (
                <tr key={a.id || i} className="border-b border-slate-50 hover:bg-slate-50/50 transition-colors">
                  <td className="table-cell text-xs">
                    <Link to={`/agents/${a.id}`} className="font-medium text-primary hover:underline">
                      {a.name || a.id?.slice(0, 24)}
                    </Link>
                  </td>
                  <td className="table-cell text-center">
                    <span className={`badge ${a.status === 'active' ? 'badge-success' : 'badge-neutral'}`}>{a.status}</span>
                  </td>
                  <td className="table-cell text-xs metric-font text-right">{fmtNum(a.total_requests)}</td>
                  <td className="table-cell text-xs metric-font text-right">{fmtUsd(a.total_cost_usd)}</td>
                  <td className="table-cell text-xs text-slate-400">{fmtDate(a.last_seen_at)}</td>
                  <td className="table-cell text-center">
                    <button
                      onClick={() => assignAgent.mutate({ agentId: a.id, projectId: null })}
                      className="text-xs text-red-400 hover:text-red-600"
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {agents.length === 0 && (
            <div className="text-center py-8 text-slate-400">
              <p className="text-sm">No agents in this project yet</p>
            </div>
          )}
        </div>

      </div>
    </div>
  )
}
