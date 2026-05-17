import { useState, useEffect, useRef, useCallback, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { AlertTriangle, Activity, CheckCircle2, GitMerge } from 'lucide-react'
import { clsx } from 'clsx'
import { EnterpriseFeatureCard } from '@/components/common/EnterpriseFeatureCard'

// ── Types ─────────────────────────────────────────────────────────────────────

type Severity = 'info' | 'warn' | 'critical'

interface AlertRow {
  id: string
  timestamp: string
  agent_id: string
  session_id?: string | null
  detector: string
  severity: Severity
  score: number
  details: unknown
  acknowledged_at?: string | null
}

// ── API helpers ───────────────────────────────────────────────────────────────

function getApiKey(): string {
  return (
    (import.meta.env.VITE_API_KEY as string | undefined) ||
    localStorage.getItem('govrix_api_key') ||
    'govrix-local-dev'
  )
}

function apiBase(): string {
  return (import.meta.env.VITE_API_URL as string | undefined) ?? ''
}

async function fetchAlerts(params: {
  severity?: Severity | ''
  detector?: string
  limit?: number
  since?: string
}): Promise<AlertRow[]> {
  const url = new URL(`${apiBase()}/api/v1/alerts`, window.location.origin)
  if (params.severity) url.searchParams.set('severity', params.severity)
  if (params.detector) url.searchParams.set('detector', params.detector)
  if (params.since) url.searchParams.set('since', params.since)
  url.searchParams.set('limit', String(params.limit ?? 200))

  const res = await fetch(url.toString().replace(window.location.origin, ''), {
    headers: { Authorization: `Bearer ${getApiKey()}` },
  })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  const body = await res.json()
  return (body?.data ?? []) as AlertRow[]
}

async function acknowledgeAlert(id: string): Promise<void> {
  await fetch(`${apiBase()}/api/v1/alerts/${id}/acknowledge`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${getApiKey()}` },
  })
}

// ── Severity badge ────────────────────────────────────────────────────────────

const SEV_STYLES: Record<Severity, { dot: string; badge: string; label: string }> = {
  info:     { dot: 'bg-slate-400',  badge: 'bg-slate-500/15 text-slate-300 ring-slate-500/30',  label: 'Info' },
  warn:     { dot: 'bg-amber-400',  badge: 'bg-amber-500/15 text-amber-300 ring-amber-500/30',  label: 'Warn' },
  critical: { dot: 'bg-rose-400',   badge: 'bg-rose-500/15 text-rose-300 ring-rose-500/30',     label: 'Critical' },
}

function SeverityBadge({ severity }: { severity: Severity }) {
  const s = SEV_STYLES[severity] ?? SEV_STYLES.info
  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[0.6875rem] font-semibold tracking-wide ring-1',
        s.badge,
      )}
    >
      <span className={clsx('w-1.5 h-1.5 rounded-full', s.dot)} />
      {s.label}
    </span>
  )
}

// ── Severity bucket strip ─────────────────────────────────────────────────────

function SeverityStrip({ rows }: { rows: AlertRow[] }) {
  const counts = useMemo(() => {
    const c = { info: 0, warn: 0, critical: 0 }
    for (const r of rows) {
      c[r.severity] = (c[r.severity] ?? 0) + 1
    }
    return c
  }, [rows])

  const items: Array<{ key: Severity; label: string; count: number; tone: string }> = [
    { key: 'critical', label: 'Critical', count: counts.critical, tone: 'text-rose-300 border-rose-500/30 bg-rose-500/10' },
    { key: 'warn',     label: 'Warn',     count: counts.warn,     tone: 'text-amber-300 border-amber-500/30 bg-amber-500/10' },
    { key: 'info',     label: 'Info',     count: counts.info,     tone: 'text-slate-300 border-slate-500/30 bg-slate-500/10' },
  ]
  return (
    <div className="grid grid-cols-3 gap-4">
      {items.map(it => (
        <div key={it.key} className={clsx('rounded-xl border px-5 py-4 flex items-center justify-between', it.tone)}>
          <div>
            <div className="text-[0.6875rem] uppercase tracking-widest opacity-70">{it.label}</div>
            <div className="font-display text-3xl font-semibold tabular-nums mt-1">{it.count}</div>
          </div>
          <AlertTriangle className="w-5 h-5 opacity-50" />
        </div>
      ))}
    </div>
  )
}

// ── Sparkline tile ────────────────────────────────────────────────────────────

interface Sparkline {
  detector: string
  total: number
  buckets: number[]
}

function buildDetectorSparklines(rows: AlertRow[], bucketCount = 24): Sparkline[] {
  const now = Date.now()
  const windowMs = 24 * 60 * 60 * 1000
  const bucketMs = windowMs / bucketCount

  const byDetector = new Map<string, number[]>()
  for (const r of rows) {
    const t = Date.parse(r.timestamp)
    if (Number.isNaN(t)) continue
    if (now - t > windowMs) continue
    const idx = Math.min(bucketCount - 1, Math.max(0, Math.floor((t - (now - windowMs)) / bucketMs)))
    if (!byDetector.has(r.detector)) byDetector.set(r.detector, new Array(bucketCount).fill(0))
    byDetector.get(r.detector)![idx] += 1
  }

  return [...byDetector.entries()]
    .map(([detector, buckets]) => ({
      detector,
      buckets,
      total: buckets.reduce((a, b) => a + b, 0),
    }))
    .sort((a, b) => b.total - a.total)
}

function SparklineSvg({ buckets }: { buckets: number[] }) {
  const max = Math.max(1, ...buckets)
  const w = 160
  const h = 36
  const step = w / Math.max(1, buckets.length - 1)
  const points = buckets
    .map((v, i) => `${(i * step).toFixed(1)},${(h - (v / max) * h).toFixed(1)}`)
    .join(' ')
  return (
    <svg width={w} height={h} className="overflow-visible">
      <polyline
        points={points}
        fill="none"
        stroke="#10b981"
        strokeWidth={1.5}
        strokeLinejoin="round"
        strokeLinecap="round"
      />
    </svg>
  )
}

function DetectorGrid({ rows }: { rows: AlertRow[] }) {
  const sparklines = useMemo(() => buildDetectorSparklines(rows), [rows])
  if (sparklines.length === 0) {
    return (
      <div className="card p-8 text-center">
        <p className="text-sm text-slate-500">No detector activity in last 24h.</p>
      </div>
    )
  }
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3">
      {sparklines.map(s => (
        <div key={s.detector} className="card-interactive p-4 flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <span className="font-mono text-xs text-slate-200">{s.detector}</span>
            <span className="text-[0.6875rem] text-slate-500 tabular-nums">{s.total} / 24h</span>
          </div>
          <SparklineSvg buckets={s.buckets} />
        </div>
      ))}
    </div>
  )
}

// ── Live SSE feed hook ────────────────────────────────────────────────────────

function useLiveAlerts(initial: AlertRow[]) {
  const [items, setItems] = useState<AlertRow[]>(initial)
  const esRef = useRef<EventSource | null>(null)
  const reconnectRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [connected, setConnected] = useState(false)

  // Sync when initial changes (refetch).
  useEffect(() => {
    setItems(initial)
  }, [initial])

  const connect = useCallback(() => {
    esRef.current?.close()
    const url = `${apiBase()}/api/v1/alerts/stream?api_key=${getApiKey()}`
    try {
      const es = new EventSource(url)
      esRef.current = es
      es.onopen = () => setConnected(true)
      es.addEventListener('alert', (e: MessageEvent) => {
        try {
          const a = JSON.parse(e.data as string) as AlertRow
          setItems(prev => [a, ...prev].slice(0, 500))
        } catch {
          // ignore
        }
      })
      es.onerror = () => {
        setConnected(false)
        es.close()
        reconnectRef.current = setTimeout(connect, 5000)
      }
    } catch {
      setConnected(false)
    }
  }, [])

  useEffect(() => {
    connect()
    return () => {
      esRef.current?.close()
      if (reconnectRef.current) clearTimeout(reconnectRef.current)
    }
  }, [connect])

  return { items, setItems, connected }
}

// ── Live feed table ───────────────────────────────────────────────────────────

function LiveFeed({
  rows,
  onAck,
}: {
  rows: AlertRow[]
  onAck: (id: string) => void
}) {
  return (
    <div className="card overflow-hidden">
      <div className="px-5 py-3 border-b border-[var(--govrix-border)] flex items-center gap-3">
        <Activity className="w-4 h-4 text-brand-400" />
        <span className="text-sm font-semibold font-display text-slate-200">Live Anomaly Feed</span>
        <span className="text-[11px] text-slate-500">{rows.length} alerts</span>
      </div>
      <div className="max-h-[500px] overflow-y-auto">
        <table className="w-full text-xs">
          <thead className="bg-white/[0.02] text-[0.625rem] uppercase tracking-widest text-slate-500">
            <tr>
              <th className="text-left px-4 py-2 font-medium">Timestamp</th>
              <th className="text-left px-4 py-2 font-medium">Detector</th>
              <th className="text-left px-4 py-2 font-medium">Severity</th>
              <th className="text-left px-4 py-2 font-medium">Agent</th>
              <th className="text-right px-4 py-2 font-medium">Score</th>
              <th className="text-right px-4 py-2 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={6} className="px-4 py-8 text-center text-slate-500 text-sm">
                  No alerts yet — waiting for detector output…
                </td>
              </tr>
            ) : (
              rows.map(r => (
                <tr
                  key={r.id}
                  className={clsx(
                    'border-t border-[var(--govrix-border)] hover:bg-white/[0.015] transition-colors',
                    r.acknowledged_at && 'opacity-50',
                  )}
                >
                  <td className="px-4 py-2 font-mono text-[11px] text-slate-500 whitespace-nowrap">
                    {r.timestamp.replace('T', ' ').replace('Z', '').slice(0, 19)}
                  </td>
                  <td className="px-4 py-2 font-mono text-[11px] text-slate-200">{r.detector}</td>
                  <td className="px-4 py-2"><SeverityBadge severity={r.severity} /></td>
                  <td className="px-4 py-2 font-mono text-[11px] text-brand-400 truncate max-w-[160px]">{r.agent_id}</td>
                  <td className="px-4 py-2 text-right font-mono tabular-nums text-slate-300">
                    {r.score.toFixed(2)}
                  </td>
                  <td className="px-4 py-2 text-right">
                    {r.acknowledged_at ? (
                      <span className="inline-flex items-center gap-1 text-[10px] text-brand-400">
                        <CheckCircle2 className="w-3 h-3" /> ack
                      </span>
                    ) : (
                      <button
                        onClick={() => onAck(r.id)}
                        className="px-2 py-0.5 text-[10px] rounded border border-slate-600 text-slate-300 hover:text-white hover:border-brand-400 transition-colors"
                      >
                        Acknowledge
                      </button>
                    )}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}

// ── Page ──────────────────────────────────────────────────────────────────────

const DETECTOR_OPTIONS = [
  'cost_zscore',
  'latency_p99',
  'exfil',
  'security',
  'fanout',
  'error_rate',
  'behavioral',
  'legacy',
]

export function AnomaliesPage() {
  const [severityFilter, setSeverityFilter] = useState<Severity | ''>('')
  const [detectorFilter, setDetectorFilter] = useState<string>('')

  const since24h = useMemo(() => new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(), [])

  const { data: alertsData, isLoading } = useQuery({
    queryKey: ['anomaly-alerts', severityFilter, detectorFilter, since24h],
    queryFn: () =>
      fetchAlerts({
        severity: severityFilter || undefined,
        detector: detectorFilter || undefined,
        since: since24h,
        limit: 500,
      }),
    staleTime: 10_000,
    retry: 1,
  })

  const { items, setItems, connected } = useLiveAlerts(alertsData ?? [])

  // Filter the live stream client-side too so newly arrived rows respect filters.
  const filtered = useMemo(() => {
    return items.filter(r => {
      if (severityFilter && r.severity !== severityFilter) return false
      if (detectorFilter && r.detector !== detectorFilter) return false
      return true
    })
  }, [items, severityFilter, detectorFilter])

  const handleAck = useCallback(
    async (id: string) => {
      await acknowledgeAlert(id)
      setItems(prev =>
        prev.map(r => (r.id === id ? { ...r, acknowledged_at: new Date().toISOString() } : r)),
      )
    },
    [setItems],
  )

  return (
    <div className="space-y-6 page-enter">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-lg font-display font-bold text-[var(--govrix-text-primary)] tracking-tight">
            Anomalies
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Detector-driven anomaly alerts &mdash; last 24 hours
          </p>
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-500 font-mono">
          <span className={clsx('w-1.5 h-1.5 rounded-full', connected ? 'bg-brand-500 pulse-glow' : 'bg-slate-500')} />
          {connected ? 'Live' : 'Offline'}
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-3 flex-wrap">
        <select
          value={severityFilter}
          onChange={e => setSeverityFilter(e.target.value as Severity | '')}
          className="bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] rounded-lg px-3 py-1.5 text-xs text-slate-200"
        >
          <option value="">All severities</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="critical">Critical</option>
        </select>
        <select
          value={detectorFilter}
          onChange={e => setDetectorFilter(e.target.value)}
          className="bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] rounded-lg px-3 py-1.5 text-xs text-slate-200"
        >
          <option value="">All detectors</option>
          {DETECTOR_OPTIONS.map(d => (
            <option key={d} value={d}>{d}</option>
          ))}
        </select>
      </div>

      {/* Severity bucket strip */}
      {isLoading ? (
        <div className="grid grid-cols-3 gap-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="skeleton h-20 rounded-xl" />
          ))}
        </div>
      ) : (
        <SeverityStrip rows={filtered} />
      )}

      {/* Detector sparkline grid */}
      <div>
        <h2 className="text-sm font-semibold text-slate-200 font-display mb-2">Detector activity (24h)</h2>
        <DetectorGrid rows={filtered} />
      </div>

      {/* Live feed */}
      <LiveFeed rows={filtered} onAck={handleAck} />

      {/* Enterprise-gated section: cross-agent correlation summary */}
      <div className="card">
        <EnterpriseFeatureCard
          icon={GitMerge}
          title="Cross-Agent Correlation requires Govrix Enterprise"
          description="Correlate anomaly clusters across agents and sessions to surface coordinated attacks, runaway sub-agents, and policy drift in real time."
        />
      </div>
    </div>
  )
}
