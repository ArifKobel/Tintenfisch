import { useWorkspaceStore, type LeftPanel } from '@/stores/workspaceStore.ts'

const TABS: { panel: LeftPanel; label: string }[] = [
  { panel: 'tools', label: 'All tools' },
  { panel: 'edit', label: 'Edit' },
]

export function PanelTabs({ className }: { className: string }) {
  const { left, showLeft } = useWorkspaceStore()
  return (
    <nav className={`panel-tabs ${className}`} aria-label="Tool panels">
      {TABS.map(({ panel, label }) => (
        <button key={panel} type="button" className={`header-tab${left === panel ? ' header-tab-active' : ''}`} aria-pressed={left === panel} onClick={() => showLeft(panel)}>
          {label}
        </button>
      ))}
    </nav>
  )
}
