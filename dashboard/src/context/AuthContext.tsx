import { createContext, useContext, useState, type ReactNode } from 'react'

type Tier = 'Scout' | 'Platform'

interface AuthContextValue {
  tier: Tier
  darkMode: boolean
  toggleDarkMode: () => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [darkMode, setDarkMode] = useState(() => {
    const saved = localStorage.getItem('govrix-dark-mode')
    return saved ? saved === 'true' : false
  })

  const toggleDarkMode = () => {
    setDarkMode(d => {
      localStorage.setItem('govrix-dark-mode', String(!d))
      return !d
    })
  }

  return (
    <AuthContext.Provider value={{ tier: 'Scout', darkMode, toggleDarkMode }}>
      <div className={darkMode ? 'dark' : ''}>{children}</div>
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used inside AuthProvider')
  return ctx
}
