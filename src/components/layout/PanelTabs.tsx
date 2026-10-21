import { useWorkspaceStore, type LeftPanel } from '@/stores/workspaceStore.ts'

const TABS: { panel: LeftPanel; label: string }[] = [
  { panel: 'tools', label: 'All tools' },
  { panel: 'edit', label: 'Edit' },
]

export function PanelTabs({ className }: { className: string }) {
  const { left, showLeft } = useWorkspaceStore()
  return (
    <nav className={`flex gap-6 self-stretch ${className}`} aria-label="Tool panels">
      {TABS.map(({ panel, label }) => (
        <button key={panel} type="button" className={`relative cursor-pointer text-ink-strong hover:text-black ${left === panel ? 'after:absolute after:inset-x-0 after:bottom-0 after:h-0.5 after:rounded-[1px] after:bg-ink-strong' : ''}`} aria-pressed={left === panel} onClick={() => showLeft(panel)}>
          {label}
        </button>
      ))}
    </nav>
  )
}
