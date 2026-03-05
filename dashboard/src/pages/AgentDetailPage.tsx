import { useParams, Link } from 'react-router-dom'
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts'
import { ArrowLeft, Bot, Shield, AlertTriangle, Clock, DollarSign, Zap } from 'lucide-react'
import { useAgent, useAgentRuns, useAgentViolations, useCostTimeseries } from '../api/hooks'

const fmtNum = (n: number | undefined | null): string =>
  typeof n === 'number' ? n.toLocaleString() : '0'

const fmtUsd = (n: number | undefined | null): string =>
  typeof n === 'number' ? `$${n.toFixed(4)}` : '$0'

const fmtDate = (s: string | undefined | null): string => {
  if (!s) return '\u2014'
  return new Date(s).toLocaleString([], {
    month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
  })
}

const statusColor = (s: string | undefined): string => {
  const map: Record<string, string> = {
    active: 'badge-success',
    idle: 'badge-neutral',
    error: 'badge-danger',
    blocked: 'badge-danger',
    retired: 'badge-neutral',
  }
  return map[s ?? ''] || 'badge-neutral'
}

export default function AgentDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { data: agentData, isLoading: agentLoading } = useAgent(id ?? '')
  const { data: runsData, isLoading: runsLoading } = useAgentRuns(id ?? '')
  const { data: violationsData } = useAgentViolations(id ?? '')
  const { data: timeseriesData } = useCostTimeseries({ agent_id: id })

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const agent = (agentData as any)?.data ?? agentData
  const runs = runsData?.data ?? []
  const violations = violationsData?.data ?? []
  const timeseries = timeseriesData?.data ?? []

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const chartData = timeseries.map((b: any) => ({
    time: b.bucket ? new Date(b.bucket).toLocaleDateString([], { month: 'short', day: 'numeric' }) : '',
    cost: b.total_cost_usd ?? 0,
    requests: b.request_count ?? 0,
  }))

  if (agentLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="animate-pulse text-slate-400 text-sm">Loading agent...</div>
      </div>
    )
  }

  if (!agent) {
    return (
      <div className="flex-1 flex items-center justify-center flex-col gap-4">
        <Bot className="w-12 h-12 text-slate-300" />
        <p className="text-sm text-slate-500">Agent not found</p>
        <Link to="/agents" className="text-xs text-primary hover:underline">Back to agents</Link>
      </div>
    )
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-4">

        {/* Back link */}
        <Link to="/agents" className="inline-flex items-center gap-1.5 text-xs text-slate-400 hover:text-primary transition-colors mb-2">
          <ArrowLeft className="w-3.5 h-3.5" />
          Back to Agents
        </Link>

        {/* Header Card */}
        <div className="bg-white border border-slate-200 rounded-xl p-5">
          <div className="flex items-start justify-between">
            <div>
              <div className="flex items-center gap-3 mb-1">
                <h2 className="text-lg font-bold text-slate-900">{agent.name || agent.id}</h2>
                <span className={`badge ${statusColor(agent.status)}`}>{agent.status || 'unknown'}</span>
              </div>
              <p className="text-xs text-slate-400 font-mono">{agent.id}</p>
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 mt-5">
            <div className="bg-slate-50 rounded-lg p-3">
              <div className="flex items-center gap-1.5 mb-1">
                <Zap className="w-3.5 h-3.5 text-primary" />
                <span className="text-[10px] uppercase tracking-wider text-slate-400 font-bold">Requests</span>
              </div>
              <p className="text-lg font-bold metric-font text-slate-800">{fmtNum(agent.total_requests)}</p>
            </div>
            <div className="bg-slate-50 rounded-lg p-3">
              <div className="flex items-center gap-1.5 mb-1">
                <DollarSign className="w-3.5 h-3.5 text-emerald-500" />
                <span className="text-[10px] uppercase tracking-wider text-slate-400 font-bold">Total Cost</span>
              </div>
              <p className="text-lg font-bold metric-font text-slate-800">{fmtUsd(agent.total_cost_usd)}</p>
            </div>
            <div className="bg-slate-50 rounded-lg p-3">
              <div className="flex items-center gap-1.5 mb-1">
                <Clock className="w-3.5 h-3.5 text-amber-500" />
                <span className="text-[10px] uppercase tracking-wider text-slate-400 font-bold">First Seen</span>
              </div>
              <p className="text-sm font-semibold text-slate-700">{fmtDate(agent.first_seen_at)}</p>
            </div>
            <div className="bg-slate-50 rounded-lg p-3">
              <div className="flex items-center gap-1.5 mb-1">
                <Clock className="w-3.5 h-3.5 text-blue-500" />
                <span className="text-[10px] uppercase tracking-wider text-slate-400 font-bold">Last Seen</span>
              </div>
              <p className="text-sm font-semibold text-slate-700">{fmtDate(agent.last_seen_at)}</p>
            </div>
          </div>
        </div>

        {/* Cost Over Time Chart */}
        {chartData.length > 0 && (
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">Cost Over Time</h3>
            <ResponsiveContainer width="100%" height={220}>
              <AreaChart data={chartData}>
                <defs>
                  <linearGradient id="costGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
                <XAxis dataKey="time" tick={{ fontSize: 10, fill: '#94a3b8' }} axisLine={false} tickLine={false} />
                <YAxis tick={{ fontSize: 10, fill: '#94a3b8' }} axisLine={false} tickLine={false} width={60}
                  tickFormatter={(v: number) => `$${v.toFixed(2)}`} />
                <Tooltip contentStyle={{ fontSize: 11, borderRadius: 8 }}
                  formatter={(value: number) => [`$${value.toFixed(4)}`, 'Cost']} />
                <Area type="monotone" dataKey="cost" stroke="#6366f1" fill="url(#costGradient)" strokeWidth={2} />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        )}

        {/* Runs Timeline */}
        <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
          <div className="p-4 border-b border-slate-100">
            <h3 className="text-sm font-bold text-slate-700">Runs (Sessions)</h3>
            <p className="text-xs text-slate-400">{runs.length} session{runs.length !== 1 ? 's' : ''}</p>
          </div>
          <table className="w-full">
            <thead>
              <tr className="border-b border-slate-100 bg-slate-50/50">
                <th className="table-header text-left py-3 px-4">Session</th>
                <th className="table-header text-left py-3 px-4">Started</th>
                <th className="table-header text-left py-3 px-4">Model</th>
                <th className="table-header text-right py-3 px-4">Events</th>
                <th className="table-header text-right py-3 px-4">Tokens In</th>
                <th className="table-header text-right py-3 px-4">Tokens Out</th>
                <th className="table-header text-right py-3 px-4">Cost</th>
                <th className="table-header text-center py-3 px-4">Violations</th>
              </tr>
            </thead>
            <tbody>
              {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
              {runs.map((run: any, i: number) => (
                <tr key={run.session_id || i} className="border-b border-slate-50 hover:bg-slate-50/50 transition-colors">
                  <td className="table-cell text-xs font-mono text-primary max-w-[140px] truncate">
                    {run.session_id?.slice(0, 8) || '\u2014'}
                  </td>
                  <td className="table-cell text-xs text-slate-500">{fmtDate(run.started_at)}</td>
                  <td className="table-cell text-xs text-slate-500">{run.model || '\u2014'}</td>
                  <td className="table-cell text-xs metric-font text-right">{fmtNum(run.event_count)}</td>
                  <td className="table-cell text-xs metric-font text-right">{fmtNum(run.total_input_tokens)}</td>
                  <td className="table-cell text-xs metric-font text-right">{fmtNum(run.total_output_tokens)}</td>
                  <td className="table-cell text-xs metric-font text-right">{fmtUsd(run.total_cost_usd)}</td>
                  <td className="table-cell text-center">
                    {run.has_violations ? (
                      <span className="badge badge-danger">Yes</span>
                    ) : (
                      <span className="badge badge-success">Clean</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {runs.length === 0 && !runsLoading && (
            <div className="text-center py-8 text-slate-400">
              <p className="text-sm">No sessions recorded yet</p>
            </div>
          )}
        </div>

        {/* Violations Panel */}
        <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
          <div className="p-4 border-b border-slate-100">
            <div className="flex items-center gap-2">
              <Shield className="w-4 h-4 text-amber-500" />
              <h3 className="text-sm font-bold text-slate-700">Violations</h3>
            </div>
            <p className="text-xs text-slate-400">{violations.length} violation{violations.length !== 1 ? 's' : ''} detected</p>
          </div>
          {violations.length > 0 ? (
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-100 bg-slate-50/50">
                  <th className="table-header text-left py-3 px-4">Time</th>
                  <th className="table-header text-left py-3 px-4">Compliance Tag</th>
                  <th className="table-header text-left py-3 px-4">Model</th>
                  <th className="table-header text-left py-3 px-4">PII Detected</th>
                  <th className="table-header text-left py-3 px-4">Error</th>
                </tr>
              </thead>
              <tbody>
                {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
                {violations.map((v: any, i: number) => (
                  <tr key={v.id || i} className="border-b border-slate-50 hover:bg-slate-50/50 transition-colors">
                    <td className="table-cell text-xs text-slate-500">{fmtDate(v.timestamp)}</td>
                    <td className="table-cell">
                      <span className={`badge ${v.compliance_tag?.startsWith('block') ? 'badge-danger' : 'badge-warning'}`}>
                        {v.compliance_tag || '\u2014'}
                      </span>
                    </td>
                    <td className="table-cell text-xs text-slate-500">{v.model || '\u2014'}</td>
                    <td className="table-cell text-xs text-slate-500">
                      {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
                      {Array.isArray(v.pii_detected) && v.pii_detected.length > 0
                        ? v.pii_detected.map((p: any) => p.pii_type || p).join(', ')
                        : '\u2014'}
                    </td>
                    <td className="table-cell text-xs text-red-400 max-w-[200px] truncate">
                      {v.error_message || '\u2014'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="text-center py-8 text-slate-400">
              <AlertTriangle className="w-8 h-8 mx-auto mb-2 text-slate-300" />
              <p className="text-sm">No violations detected</p>
            </div>
          )}
        </div>

      </div>
    </div>
  )
}
