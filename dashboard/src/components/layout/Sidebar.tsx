import { NavLink } from 'react-router-dom'
import {
  LayoutDashboard,
  Bot,
  Activity,
  DollarSign,
  FileText,
  Settings,
  ExternalLink,
  Zap,
} from 'lucide-react'
import { clsx } from 'clsx'

const NAV_ITEMS = [
  { to: '/overview', label: 'Overview',  icon: LayoutDashboard },
  { to: '/agents',   label: 'Agents',    icon: Bot },
  { to: '/events',   label: 'Events',    icon: Activity },
  { to: '/costs',    label: 'Costs',     icon: DollarSign },
  { to: '/reports',  label: 'Reports',   icon: FileText },
  { to: '/settings', label: 'Settings',  icon: Settings },
]

export function Sidebar() {
  return (
    <aside
      className="w-60 h-screen flex flex-col shrink-0"
      style={{
        background: 'rgba(5, 9, 17, 0.9)',
        backdropFilter: 'blur(20px) saturate(160%)',
        WebkitBackdropFilter: 'blur(20px) saturate(160%)',
        borderRight: '1px solid rgba(148,163,184,0.07)',
      }}
    >
      {/* Brand */}
      <div
        className="px-5 py-5"
        style={{ borderBottom: '1px solid rgba(148,163,184,0.07)' }}
      >
        <div className="flex items-center gap-2.5">
          <div
            className="flex items-center justify-center w-8 h-8 rounded-lg shrink-0"
            style={{
              background: 'linear-gradient(135deg, #0ea5e9 0%, #0369a1 100%)',
              boxShadow: '0 0 16px rgba(14,165,233,0.28), 0 2px 8px rgba(0,0,0,0.3)',
            }}
          >
            <Zap className="w-4 h-4 text-white" />
          </div>
          <div>
            <div className="font-display text-sm font-bold text-white tracking-tight">Scout</div>
            <div
              className="text-[10px] text-slate-500 -mt-0.5"
              style={{ fontFamily: 'JetBrains Mono' }}
            >
              by Govrix
            </div>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 py-4 space-y-px">
        {NAV_ITEMS.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              clsx(
                'group relative flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200',
                isActive
                  ? 'text-brand-300'
                  : 'text-slate-500 hover:text-slate-200 hover:bg-white/[0.03]',
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && (
                  <div
                    className="absolute inset-0 rounded-lg"
                    style={{
                      background: 'rgba(14,165,233,0.08)',
                      borderLeft: '2px solid #0ea5e9',
                    }}
                  />
                )}
                <Icon
                  className={clsx(
                    'w-4 h-4 shrink-0 transition-colors duration-150 relative z-10',
                    isActive ? 'text-brand-400' : 'text-slate-600 group-hover:text-slate-400',
                  )}
                />
                <span className="relative z-10">{label}</span>
              </>
            )}
          </NavLink>
        ))}
      </nav>

      {/* Footer */}
      <div
        className="px-3 py-4 space-y-2"
        style={{ borderTop: '1px solid rgba(148,163,184,0.07)' }}
      >
        <a
          href="https://govrix.io/platform"
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-semibold transition-all duration-200"
          style={{
            color: '#38bdf8',
            background: 'rgba(14,165,233,0.08)',
            border: '1px solid rgba(14,165,233,0.2)',
          }}
          onMouseEnter={e => {
            const el = e.currentTarget as HTMLAnchorElement
            el.style.background = 'rgba(14,165,233,0.14)'
            el.style.boxShadow = '0 0 16px rgba(14,165,233,0.12)'
          }}
          onMouseLeave={e => {
            const el = e.currentTarget as HTMLAnchorElement
            el.style.background = 'rgba(14,165,233,0.08)'
            el.style.boxShadow = ''
          }}
        >
          <Zap className="w-3.5 h-3.5" />
          Upgrade to Platform
          <ExternalLink className="w-3 h-3 ml-auto" />
        </a>
        <div
          className="px-3 text-[10px] text-slate-700"
          style={{ fontFamily: 'JetBrains Mono' }}
        >
          Govrix Scout OSS
        </div>
      </div>
    </aside>
  )
}
