import { useHealth, useCostSummary, useAgents, useEvents } from '../api/hooks'
import LoadingState from '../components/common/LoadingState'
import { Activity, Users, DollarSign, Zap } from 'lucide-react'
import type { ElementType } from 'react'

function StatCard({ label, value, icon: Icon, sub }: {
  label: string; value: string; icon: ElementType; sub?: string
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
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard label="Active Agents" value={String(activeAgents)} icon={Users} sub={`${agents?.total ?? 0} total registered`} />
        <StatCard label="Total Requests" value={totalRequests} icon={Activity} sub="all time" />
        <StatCard label="Total Cost" value={`$${totalCost}`} icon={DollarSign} sub="USD, all time" />
        <StatCard label="Backend" value={health?.status ?? '—'} icon={Zap} sub={`v${health?.version ?? '?'}`} />
      </div>

      <div className="card">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Recent Events</h2>
        {!events?.data.length ? (
          <p className="text-sm text-slate-400 text-center py-8">No events yet — traffic will appear here</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['Agent', 'Kind', 'Model', 'Cost', 'Time'].map(h => (
                  <th key={h} className="pb-2 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/50">
              {events.data.map(e => (
                <tr key={e.id} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2 text-slate-700 dark:text-slate-300 font-mono text-xs truncate max-w-[120px]">{e.agent_id.slice(0, 12)}</td>
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
