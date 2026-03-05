import { useState } from 'react'
import { Link } from 'react-router-dom'
import { FolderOpen, Plus, X } from 'lucide-react'
import { useProjects, useCreateProject } from '../api/hooks'

const fmtUsd = (n: number | undefined | null): string =>
  typeof n === 'number' ? `$${n.toFixed(2)}` : '$0'

export default function ProjectsPage() {
  const { data, isLoading } = useProjects()
  const createProject = useCreateProject()
  const [showCreate, setShowCreate] = useState(false)
  const [form, setForm] = useState({ name: '', description: '' })

  const projects = data?.data ?? []

  const handleCreate = async () => {
    if (!form.name.trim()) return
    await createProject.mutateAsync({ name: form.name, description: form.description || undefined })
    setShowCreate(false)
    setForm({ name: '', description: '' })
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-4">

        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Projects</h2>
            <p className="text-xs text-slate-400">{projects.length} project{projects.length !== 1 ? 's' : ''}</p>
          </div>
          <button
            onClick={() => setShowCreate(true)}
            className="btn-primary flex items-center gap-1.5 text-xs"
          >
            <Plus className="w-3.5 h-3.5" />
            New Project
          </button>
        </div>

        {/* Create Modal */}
        {showCreate && (
          <div className="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
            <div className="bg-white rounded-xl border border-slate-200 p-6 w-full max-w-md shadow-xl">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-sm font-bold text-slate-700">Create Project</h3>
                <button onClick={() => setShowCreate(false)} className="text-slate-400 hover:text-slate-600">
                  <X className="w-4 h-4" />
                </button>
              </div>
              <div className="space-y-3">
                <input
                  className="input-field w-full text-sm"
                  placeholder="Project name"
                  value={form.name}
                  onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
                  autoFocus
                />
                <textarea
                  className="input-field w-full text-sm resize-none"
                  placeholder="Description (optional)"
                  rows={3}
                  value={form.description}
                  onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
                />
                <div className="flex justify-end gap-2 mt-4">
                  <button onClick={() => setShowCreate(false)} className="btn-secondary text-xs">Cancel</button>
                  <button
                    onClick={handleCreate}
                    disabled={!form.name.trim() || createProject.isPending}
                    className="btn-primary text-xs disabled:opacity-50"
                  >
                    {createProject.isPending ? 'Creating...' : 'Create'}
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Project Cards */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {projects.map((p: any) => (
            <Link
              key={p.id}
              to={`/projects/${p.id}`}
              className="bg-white border border-slate-200 rounded-xl p-5 hover:border-primary/30 hover:shadow-sm transition-all"
            >
              <div className="flex items-start gap-3 mb-3">
                <div className="w-9 h-9 rounded-lg bg-primary/10 flex items-center justify-center">
                  <FolderOpen className="w-4.5 h-4.5 text-primary" />
                </div>
                <div className="flex-1 min-w-0">
                  <h3 className="text-sm font-bold text-slate-800 truncate">{p.name}</h3>
                  {p.description && (
                    <p className="text-xs text-slate-400 mt-0.5 line-clamp-2">{p.description}</p>
                  )}
                </div>
              </div>
              <div className="flex items-center justify-between text-xs text-slate-500 mt-3 pt-3 border-t border-slate-100">
                <span>{p.agent_count ?? 0} agent{(p.agent_count ?? 0) !== 1 ? 's' : ''}</span>
                <span className="metric-font font-semibold">{fmtUsd(p.total_cost_usd)}</span>
              </div>
            </Link>
          ))}
        </div>

        {projects.length === 0 && !isLoading && (
          <div className="text-center py-12 text-slate-400">
            <FolderOpen className="w-10 h-10 mx-auto mb-3 text-slate-300" />
            <p className="text-sm font-medium">No projects yet</p>
            <p className="text-xs mt-1">Create a project to group related agents together</p>
          </div>
        )}

      </div>
    </div>
  )
}
