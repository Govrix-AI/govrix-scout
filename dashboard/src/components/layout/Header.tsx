import { useLocation } from 'react-router-dom'
import { AlertTriangle, XCircle, Loader2 } from 'lucide-react'
import { useHealth } from '@/api/hooks'

const PAGE_TITLES: Record<string, string> = {
  '/overview': 'Overview',
  '/agents':   'Agents',
  '/events':   'Events',
  '/costs':    'Costs',
  '/reports':  'Reports',
  '/settings': 'Settings',
}

const PAGE_DESCRIPTIONS: Record<string, string> = {
  '/overview': 'System health and key metrics',
  '/agents':   'Registered AI agents',
  '/events':   'Request and response activity',
  '/costs':    'Token usage and spend tracking',
  '/reports':  'Generated compliance reports',
  '/settings': 'Platform configuration',
}

function HealthIndicator() {
  const { data, isLoading, isError } = useHealth()

  if (isLoading) {
    return (
      <div className="flex items-center gap-1.5 text-xs text-slate-600">
        <Loader2 className="w-3.5 h-3.5 animate-spin" />
        <span style={{ fontFamily: 'JetBrains Mono' }}>Connecting</span>
      </div>
    )
  }

  if (isError) {
    return (
      <div className="flex items-center gap-1.5 text-xs text-rose-500">
        <XCircle className="w-3.5 h-3.5" />
        <span style={{ fontFamily: 'JetBrains Mono' }}>API Offline</span>
      </div>
    )
  }

  if (data?.status === 'ok') {
    return (
      <div className="flex items-center gap-2.5 text-xs">
        <div className="flex items-center gap-2">
          <div className="relative w-1.5 h-1.5">
            <div className="absolute inset-0 rounded-full bg-brand-400" />
            <div className="absolute inset-0 rounded-full bg-brand-400 pulse-glow" />
          </div>
          <span style={{ fontFamily: 'JetBrains Mono' }} className="text-brand-400">
            Live
          </span>
          {data.version && (
            <span style={{ fontFamily: 'JetBrains Mono' }} className="text-slate-700">
              v{data.version}
            </span>
          )}
        </div>
      </div>
    )
  }

  return (
    <div className="flex items-center gap-1.5 text-xs text-yellow-400">
      <AlertTriangle className="w-3.5 h-3.5" />
      <span style={{ fontFamily: 'JetBrains Mono' }}>Degraded</span>
    </div>
  )
}

export function Header() {
  const location = useLocation()
  const match = Object.keys(PAGE_TITLES).find(path => location.pathname.startsWith(path))
  const title = match ? PAGE_TITLES[match] : 'Scout'
  const description = match ? PAGE_DESCRIPTIONS[match] : ''

  return (
    <header
      className="h-14 flex items-center justify-between px-6 shrink-0"
      style={{
        background: 'rgba(5, 9, 17, 0.8)',
        backdropFilter: 'blur(16px)',
        WebkitBackdropFilter: 'blur(16px)',
        borderBottom: '1px solid rgba(148, 163, 184, 0.07)',
      }}
    >
      <div className="flex items-baseline gap-3">
        <h1 className="font-display text-[0.9375rem] font-semibold text-slate-100 tracking-tight">
          {title}
        </h1>
        {description && (
          <span className="text-xs text-slate-600 hidden sm:inline">{description}</span>
        )}
      </div>
      <HealthIndicator />
    </header>
  )
}
