import { useState } from 'react'
import { TOOL_CATALOG } from '@/components/tools/toolCatalog.ts'
import { Icon, type IconName } from '@/components/ui/Icon.tsx'
import { PanelItem } from '@/components/ui/PanelItem.tsx'
import { SidePanel } from '@/components/ui/SidePanel.tsx'
import { useDigitalSignatureStore } from '@/stores/digitalSignatureStore.ts'
import { usePagesStore } from '@/stores/pagesStore.ts'
import { usePrintStore } from '@/stores/printStore.ts'
import { useProtectionStore } from '@/stores/protectionStore.ts'
import { useSignatureStore } from '@/stores/signatureStore.ts'
import { useWorkspaceStore, type Tool } from '@/stores/workspaceStore.ts'

interface Action {
  label: string
  icon: IconName
  run: () => void
}

type Entry = Tool | Action

const GROUPS: { title: string; entries: Entry[] }[] = [
  {
    title: 'View',
    entries: [
      { label: 'Comments', icon: 'comment', run: () => useWorkspaceStore.getState().showRight('comments') },
      { label: 'Pages', icon: 'view-grid', run: () => useWorkspaceStore.getState().showRight('pages') },
      { label: 'Bookmarks', icon: 'bookmark', run: () => useWorkspaceStore.getState().showRight('bookmarks') },
    ],
  },
  { title: 'Edit', entries: ['edit', 'addText', 'addImage', 'link'] },
  { title: 'Comment', entries: ['note', 'textBox', 'highlight', 'ink', 'rectangle', 'ellipse', 'arrow', 'stamp'] },
  {
    title: 'Organize pages',
    entries: [
      { label: 'Insert blank page', icon: 'file-add', run: () => void usePagesStore.getState().insertBlank() },
    ],
  },
  { title: 'Prepare form', entries: ['form'] },
  {
    title: 'Sign',
    entries: [
      'sign',
      { label: 'Create signature', icon: 'draw', run: () => useSignatureStore.getState().openPad() },
      { label: 'Sign with certificate', icon: 'badge-verified', run: () => useDigitalSignatureStore.getState().openDialog() },
    ],
  },
  { title: 'Redact', entries: ['redactText', 'redactArea'] },
  {
    title: 'Protect',
    entries: [{ label: 'Protect with password', icon: 'lock', run: () => void useProtectionStore.getState().openDialog() }],
  },
  {
    title: 'Print',
    entries: [{ label: 'Print', icon: 'print', run: () => usePrintStore.getState().openDialog() }],
  },
]

export function AllToolsPanel() {
  const { tool, choose: chooseTool, closeSheet } = useWorkspaceStore()
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set())

  function choose(entry: Entry) {
    if (typeof entry === 'string') chooseTool(entry)
    else entry.run()
    closeSheet()
  }

  function toggle(title: string) {
    const next = new Set(collapsed)
    if (!next.delete(title)) next.add(title)
    setCollapsed(next)
  }

  return (
    <SidePanel title="All tools" side="left">
      {GROUPS.map(({ title, entries }) => (
        <section key={title} className="flex flex-col border-b border-separator pb-2">
          <button type="button" className="flex min-h-10 cursor-pointer items-center gap-2 p-0 text-[13px] font-bold text-ink-strong" aria-expanded={!collapsed.has(title)} onClick={() => toggle(title)}>
            <Icon name={collapsed.has(title) ? 'chevron-right' : 'chevron-down'} className="size-3.5" />
            {title}
          </button>
          {!collapsed.has(title) &&
            entries.map((entry) =>
              typeof entry === 'string' ? (
                <PanelItem key={entry} icon={TOOL_CATALOG[entry].icon} label={TOOL_CATALOG[entry].label} chosen={entry === tool} onClick={() => choose(entry)} />
              ) : (
                <PanelItem key={entry.label} icon={entry.icon} label={entry.label} onClick={() => choose(entry)} />
              ),
            )}
        </section>
      ))}
    </SidePanel>
  )
}
