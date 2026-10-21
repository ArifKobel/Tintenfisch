import { useApplication } from '@/app/ApplicationContext.ts'
import type { LeftPanel } from '@/stores/PanelStore.ts'
import { usePanelState } from '@/hooks/usePanelState.ts'

const TABS: { panel: LeftPanel; label: string }[] = [
  { panel: 'tools', label: 'All tools' },
  { panel: 'edit', label: 'Edit' },
]

export function PanelTabs({ className }: { className: string }) {
  const { panels } = useApplication()
  const { left } = usePanelState()
  return (
    <nav className={`panel-tabs ${className}`} aria-label="Tool panels">
      {TABS.map(({ panel, label }) => (
        <button key={panel} type="button" className={`header-tab${left === panel ? ' header-tab-active' : ''}`} aria-pressed={left === panel} onClick={() => panels.showLeft(panel)}>
          {label}
        </button>
      ))}
    </nav>
  )
}
