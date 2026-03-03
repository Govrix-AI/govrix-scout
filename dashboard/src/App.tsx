import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { AuthProvider } from './context/AuthContext'
import Layout from './components/layout/Layout'
import OverviewPage from './pages/OverviewPage'
import EventsPage from './pages/EventsPage'
import AgentsPage from './pages/AgentsPage'
import CostsPage from './pages/CostsPage'
import ReportsPage from './pages/ReportsPage'
import EnterpriseGate from './components/EnterpriseGate'

const GATED = [
  {
    path: 'governance',
    description: 'Upgrade to Govrix Platform to unlock risk posture dashboards, policy enforcement, kill switches, and PII detection activity.',
  },
  {
    path: 'sessions',
    description: 'Upgrade to Govrix Platform to unlock real-time session recording, forensic replay, and compliance evidence capture.',
  },
  {
    path: 'compliance',
    description: 'Upgrade to Govrix Platform to unlock automated compliance reporting for SOC 2, EU AI Act, HIPAA, and NIST.',
  },
  {
    path: 'settings',
    description: 'Upgrade to Govrix Platform to manage system configuration, SIEM connectors, mTLS certificates, and multi-tenancy.',
  },
]

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Navigate to="/overview" replace />} />
            <Route path="overview" element={<OverviewPage />} />
            <Route path="events" element={<EventsPage />} />
            <Route path="agents" element={<AgentsPage />} />
            <Route path="costs" element={<CostsPage />} />
            <Route path="reports" element={<ReportsPage />} />
            {GATED.map(({ path, description }) => (
              <Route
                key={path}
                path={path}
                element={<EnterpriseGate feature={path} description={description} />}
              />
            ))}
            <Route path="*" element={<Navigate to="/overview" replace />} />
          </Route>
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  )
}
