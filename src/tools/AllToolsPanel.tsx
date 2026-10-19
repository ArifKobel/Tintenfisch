import { useState } from 'react'
import type { Application } from '@app/Application.ts'
import { useApplication } from '@app/ApplicationContext.ts'
import { TOOL_CATALOG } from '@tools/ToolCatalog.ts'
import type { Tool } from '@tools/ToolStore.ts'
import { useToolState } from '@tools/useToolState.ts'
import { Icon, type IconName } from '@ui/Icon.tsx'
import { PanelItem } from '@ui/PanelItem.tsx'
import { SidePanel } from '@ui/SidePanel.tsx'

interface Action {
  label: string
  icon: IconName
  run: (application: Application) => void
}

type Entry = Tool | Action

const GROUPS: { title: string; entries: Entry[] }[] = [
  { title: 'Edit', entries: ['edit', 'addText', 'addImage', 'link'] },
  { title: 'Comment', entries: ['note', 'textBox', 'highlight', 'ink', 'rectangle', 'ellipse', 'arrow', 'stamp'] },
  {
    title: 'Organize pages',
    entries: [
      { label: 'Show pages', icon: 'view-grid', run: ({ panels }) => panels.showRight('pages') },
      { label: 'Insert blank page', icon: 'file-add', run: ({ organize }) => void organize.insertBlank() },
    ],
  },
  { title: 'Prepare form', entries: ['form'] },
  {
    title: 'Sign',
    entries: [
      'sign',
      { label: 'Create signature', icon: 'draw', run: ({ signing }) => signing.openPad() },
      { label: 'Sign with certificate', icon: 'badge-verified', run: ({ digitalSignatures }) => digitalSignatures.openDialog() },
    ],
  },
  { title: 'Redact', entries: ['redactText', 'redactArea'] },
  {
    title: 'Protect',
    entries: [{ label: 'Protect with password', icon: 'lock', run: ({ protection }) => void protection.openDialog() }],
  },
  {
    title: 'Print',
    entries: [{ label: 'Print', icon: 'print', run: ({ printing }) => printing.openDialog() }],
  },
]

export function AllToolsPanel() {
  const application = useApplication()
  const { tool } = useToolState()
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set())

  function toggle(title: string) {
    const next = new Set(collapsed)
    if (!next.delete(title)) next.add(title)
    setCollapsed(next)
  }

  return (
    <SidePanel title="All tools" side="left" onClose={() => application.panels.showLeft(null)}>
      {GROUPS.map(({ title, entries }) => (
        <section key={title} className="panel-group">
          <button type="button" className="panel-group-title" aria-expanded={!collapsed.has(title)} onClick={() => toggle(title)}>
            <Icon name={collapsed.has(title) ? 'chevron-right' : 'chevron-down'} />
            {title}
          </button>
          {!collapsed.has(title) &&
            entries.map((entry) =>
              typeof entry === 'string' ? (
                <PanelItem key={entry} icon={TOOL_CATALOG[entry].icon} label={TOOL_CATALOG[entry].label} chosen={entry === tool} onClick={() => application.tools.choose(entry)} />
              ) : (
                <PanelItem key={entry.label} icon={entry.icon} label={entry.label} onClick={() => entry.run(application)} />
              ),
            )}
        </section>
      ))}
    </SidePanel>
  )
}
