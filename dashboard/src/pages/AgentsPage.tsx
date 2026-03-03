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
      {!data?.data.length ? (
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
              {data.data.map(agent => (
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
