import { useState } from 'react'
import {
  BarChart, Bar,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Cell,
} from 'recharts'
import { Wallet, RefreshCw, AlertTriangle, Settings2, Trash2, Save } from 'lucide-react'
import { useBudgetOverview, useSetAgentBudget, useDeleteAgentBudget } from '../api/hooks'
import type { BudgetOverviewRow, SetBudgetRequest } from '../api/types'

const fmtUsd = (n: number | undefined | null): string =>
  typeof n === 'number' ? `$${n.toFixed(4)}` : '$0.00'

const fmtTokens = (n: number | undefined | null): string =>
  typeof n === 'number' ? n.toLocaleString() : '0'

const pct = (used: number, limit: number | null): number | null =>
  limit != null && limit > 0 ? Math.min((used / limit) * 100, 100) : null

const barColor = (p: number | null): string =>
  p == null ? '#94a3b8' : p >= 90 ? '#ef4444' : p >= 70 ? '#f59e0b' : '#10b981'

// ── Edit Modal ────────────────────────────────────────────────────────────────

interface EditModalProps {
  row: BudgetOverviewRow
  onClose: () => void
}

function EditBudgetModal({ row, onClose }: EditModalProps) {
  const [dailyTokens, setDailyTokens] = useState(row.daily_token_limit?.toString() ?? '')
  const [dailyCost, setDailyCost] = useState(row.daily_cost_limit_usd?.toString() ?? '')
  const [monthlyCost, setMonthlyCost] = useState(row.monthly_cost_limit_usd?.toString() ?? '')

  const setBudget = useSetAgentBudget()
  const deleteBudget = useDeleteAgentBudget()

  const handleSave = () => {
    const body: SetBudgetRequest = {
      daily_token_limit: dailyTokens ? parseInt(dailyTokens, 10) : null,
      daily_cost_limit_usd: dailyCost ? parseFloat(dailyCost) : null,
      monthly_cost_limit_usd: monthlyCost ? parseFloat(monthlyCost) : null,
    }
    setBudget.mutate({ id: row.agent_id, body }, { onSuccess: onClose })
  }

  const handleDelete = () => {
    deleteBudget.mutate(row.agent_id, { onSuccess: onClose })
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm" onClick={onClose}>
      <div
        className="bg-white border border-slate-200 rounded-xl shadow-xl w-full max-w-md p-6 space-y-5"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-bold text-slate-900">
            Budget Limits &mdash; {row.name || row.agent_id}
          </h3>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 text-sm">
            &times;
          </button>
        </div>

        <div className="space-y-3">
          <label className="block">
            <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">Daily Token Limit</span>
            <input
              type="number"
              value={dailyTokens}
              onChange={e => setDailyTokens(e.target.value)}
              placeholder="No limit"
              className="mt-1 w-full border border-slate-200 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-primary/30 focus:border-primary/50 outline-none"
            />
          </label>
          <label className="block">
            <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">Daily Cost Limit (USD)</span>
            <input
              type="number"
              step="0.01"
              value={dailyCost}
              onChange={e => setDailyCost(e.target.value)}
              placeholder="No limit"
              className="mt-1 w-full border border-slate-200 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-primary/30 focus:border-primary/50 outline-none"
            />
          </label>
          <label className="block">
            <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">Monthly Cost Limit (USD)</span>
            <input
              type="number"
              step="0.01"
              value={monthlyCost}
              onChange={e => setMonthlyCost(e.target.value)}
              placeholder="No limit"
              className="mt-1 w-full border border-slate-200 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-primary/30 focus:border-primary/50 outline-none"
            />
          </label>
        </div>

        <div className="flex items-center justify-between pt-2">
          <button
            onClick={handleDelete}
            disabled={deleteBudget.isPending}
            className="flex items-center gap-1.5 text-xs text-red-500 hover:text-red-700 transition-colors disabled:opacity-50"
          >
            <Trash2 className="w-3.5 h-3.5" /> Remove Limits
          </button>
          <div className="flex gap-2">
            <button onClick={onClose} className="btn-secondary text-xs px-3 py-1.5">Cancel</button>
            <button
              onClick={handleSave}
              disabled={setBudget.isPending}
              className="btn-primary flex items-center gap-1.5 text-xs px-3 py-1.5 disabled:opacity-50"
            >
              <Save className="w-3.5 h-3.5" /> Save
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

// ── Progress Bar ──────────────────────────────────────────────────────────────

function ProgressBar({ used, limit, label }: { used: number; limit: number | null; label: string }) {
  const p = pct(used, limit)
  if (p == null) return <span className="text-[10px] text-slate-300 metric-font">no limit</span>
  return (
    <div className="min-w-[100px]">
      <div className="flex items-center justify-between mb-0.5">
        <span className="text-[9px] text-slate-400">{label}</span>
        <span className="text-[9px] metric-font text-slate-500">{p.toFixed(0)}%</span>
      </div>
      <div className="h-1.5 bg-slate-100 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${p}%`, backgroundColor: barColor(p) }}
        />
      </div>
    </div>
  )
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export default function BudgetsPage() {
  const { data, refetch } = useBudgetOverview()
  const [editing, setEditing] = useState<BudgetOverviewRow | null>(null)

  const rows: BudgetOverviewRow[] = data?.data ?? []

  // Stats
  const withLimits = rows.filter(r => r.daily_token_limit != null || r.daily_cost_limit_usd != null || r.monthly_cost_limit_usd != null)
  const overBudget = rows.filter(r => {
    const tp = pct(r.tokens_used_today, r.daily_token_limit)
    const cp = pct(r.cost_used_today, r.daily_cost_limit_usd)
    return (tp != null && tp >= 90) || (cp != null && cp >= 90)
  })
  const totalSpendToday = rows.reduce((s, r) => s + (r.cost_used_today ?? 0), 0)

  // Chart data: top 10 agents by cost
  const chartData = [...rows]
    .sort((a, b) => (b.cost_used_today ?? 0) - (a.cost_used_today ?? 0))
    .slice(0, 10)
    .map(r => {
      const cp = pct(r.cost_used_today, r.daily_cost_limit_usd)
      return {
        name: (r.name || r.agent_id || '').slice(0, 20),
        cost: r.cost_used_today ?? 0,
        fill: barColor(cp),
      }
    })

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-6">

        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Budget Management</h2>
            <p className="text-xs text-slate-400">Configure and monitor per-agent spending limits</p>
          </div>
          <button
            onClick={() => refetch()}
            className="btn-secondary flex items-center gap-1.5 text-xs"
          >
            <RefreshCw className="w-3.5 h-3.5" /> Refresh
          </button>
        </div>

        {/* KPI Row */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="stat-card">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">Total Agents</span>
              <Wallet className="w-4 h-4 text-primary" />
            </div>
            <div className="text-2xl font-black text-slate-900 metric-font">{rows.length}</div>
          </div>
          <div className="stat-card">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">With Limits</span>
              <Settings2 className="w-4 h-4 text-primary" />
            </div>
            <div className="text-2xl font-black text-slate-900 metric-font">{withLimits.length}</div>
            <p className="text-[10px] text-slate-400 mt-1">of {rows.length} agents</p>
          </div>
          <div className="stat-card">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">Near/Over Limit</span>
              <AlertTriangle className="w-4 h-4 text-amber-500" />
            </div>
            <div className="text-2xl font-black text-slate-900 metric-font">{overBudget.length}</div>
            <p className="text-[10px] text-slate-400 mt-1">&ge;90% of daily limit</p>
          </div>
          <div className="stat-card">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">Today's Spend</span>
              <Wallet className="w-4 h-4 text-primary" />
            </div>
            <div className="text-2xl font-black text-slate-900 metric-font">{fmtUsd(totalSpendToday)}</div>
            <p className="text-[10px] text-slate-400 mt-1">Across all agents</p>
          </div>
        </div>

        {/* Chart */}
        {chartData.length > 0 && (
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">Top Agents by Today's Cost</h3>
            <ResponsiveContainer width="100%" height={220}>
              <BarChart data={chartData} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" horizontal={false} />
                <XAxis
                  type="number"
                  tick={{ fontSize: 10, fill: '#94a3b8' }}
                  axisLine={false}
                  tickLine={false}
                  tickFormatter={(v: number) => `$${v.toFixed(3)}`}
                />
                <YAxis
                  type="category"
                  dataKey="name"
                  tick={{ fontSize: 10, fill: '#94a3b8' }}
                  axisLine={false}
                  tickLine={false}
                  width={140}
                />
                <Tooltip
                  contentStyle={{ fontSize: 11, borderRadius: 8, border: '1px solid #e2e8f0' }}
                  formatter={(v: number) => [`$${v.toFixed(5)}`, 'Cost Today']}
                />
                <Bar dataKey="cost" radius={[0, 4, 4, 0]}>
                  {chartData.map((entry, i) => (
                    <Cell key={i} fill={entry.fill} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}

        {/* Table */}
        <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
          <div className="px-5 py-4 border-b border-slate-100">
            <h3 className="text-sm font-bold text-slate-700">All Agents &mdash; Budget Overview</h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-slate-100">
                  <th className="text-left px-4 py-2.5 text-[10px] uppercase tracking-widest text-slate-400 font-bold">Agent</th>
                  <th className="text-left px-4 py-2.5 text-[10px] uppercase tracking-widest text-slate-400 font-bold">Status</th>
                  <th className="text-right px-4 py-2.5 text-[10px] uppercase tracking-widest text-slate-400 font-bold">Tokens Today</th>
                  <th className="text-center px-4 py-2.5 text-[10px] uppercase tracking-widest text-slate-400 font-bold">Token Limit</th>
                  <th className="text-right px-4 py-2.5 text-[10px] uppercase tracking-widest text-slate-400 font-bold">Cost Today</th>
                  <th className="text-center px-4 py-2.5 text-[10px] uppercase tracking-widest text-slate-400 font-bold">Cost Limit</th>
                  <th className="text-center px-4 py-2.5 text-[10px] uppercase tracking-widest text-slate-400 font-bold">Actions</th>
                </tr>
              </thead>
              <tbody>
                {rows.map(row => {
                  const tokP = pct(row.tokens_used_today, row.daily_token_limit)
                  const costP = pct(row.cost_used_today, row.daily_cost_limit_usd)
                  const isHot = (tokP != null && tokP >= 90) || (costP != null && costP >= 90)
                  return (
                    <tr
                      key={row.agent_id}
                      className={`border-b border-slate-50 hover:bg-slate-50/50 transition-colors ${isHot ? 'bg-red-50/30' : ''}`}
                    >
                      <td className="px-4 py-2.5">
                        <div className="font-medium text-slate-700">{row.name || row.agent_id}</div>
                        {row.name && (
                          <div className="text-[10px] text-slate-400 metric-font truncate max-w-[180px]">{row.agent_id}</div>
                        )}
                      </td>
                      <td className="px-4 py-2.5">
                        <span className={`inline-flex items-center gap-1 text-[10px] font-bold px-2 py-0.5 rounded-full ${
                          row.status === 'active' ? 'bg-emerald-50 text-emerald-600' :
                          row.status === 'blocked' ? 'bg-red-50 text-red-600' :
                          'bg-slate-100 text-slate-500'
                        }`}>
                          {row.status}
                        </span>
                      </td>
                      <td className="px-4 py-2.5 text-right metric-font text-slate-600">
                        {fmtTokens(row.tokens_used_today)}
                      </td>
                      <td className="px-4 py-2.5">
                        <ProgressBar used={row.tokens_used_today} limit={row.daily_token_limit} label="tokens" />
                      </td>
                      <td className="px-4 py-2.5 text-right metric-font text-slate-600">
                        {fmtUsd(row.cost_used_today)}
                      </td>
                      <td className="px-4 py-2.5">
                        <ProgressBar used={row.cost_used_today} limit={row.daily_cost_limit_usd} label="cost" />
                      </td>
                      <td className="px-4 py-2.5 text-center">
                        <button
                          onClick={() => setEditing(row)}
                          className="text-primary hover:text-primary/80 transition-colors"
                          title="Edit budget limits"
                        >
                          <Settings2 className="w-4 h-4 inline" />
                        </button>
                      </td>
                    </tr>
                  )
                })}
                {rows.length === 0 && (
                  <tr>
                    <td colSpan={7} className="text-center py-12 text-slate-400">
                      No agents found. Agents appear after the first proxied request.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>

      </div>

      {/* Edit Modal */}
      {editing && <EditBudgetModal row={editing} onClose={() => setEditing(null)} />}
    </div>
  )
}
