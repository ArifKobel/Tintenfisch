import { SignMenuButton } from '@/components/signing/SignMenuButton.tsx'
import { ToolGroupButton } from '@/components/tools/ToolGroupButton.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore, type Tool } from '@/stores/workspaceStore.ts'

const GROUPS: { label: string; tools: readonly Tool[] }[] = [
  { label: 'Select', tools: ['select'] },
  { label: 'Edit text and images', tools: ['edit'] },
  { label: 'Add a comment', tools: ['note', 'textBox', 'stamp'] },
  { label: 'Highlight text', tools: ['highlight'] },
  { label: 'Draw', tools: ['ink', 'rectangle', 'ellipse', 'arrow'] },
  { label: 'Prepare form', tools: ['form'] },
]

export function QuickToolbar() {
  const { tool, left, sheet, openSheet, closeSheet } = useWorkspaceStore()
  const phase = useDocumentStore((state) => state.document.phase)
  if (phase !== 'ready') return null
  return (
    <div className="quick-toolbar-gutter">
      <div className="quick-toolbar" role="toolbar" aria-label="Quick tools" aria-orientation="vertical">
        {GROUPS.map((group) => (
          <ToolGroupButton key={group.label} label={group.label} tools={group.tools} current={tool} />
        ))}
        <SignMenuButton />
        <span className="sheet-toggle">
          <IconButton
            icon={left === 'edit' ? 'text' : 'tools'}
            label={left === 'edit' ? 'Edit panel' : 'All tools'}
            tooltip={null}
            look={sheet ? 'toggled' : null}
            pressed={sheet}
            onClick={sheet ? closeSheet : openSheet}
          />
        </span>
      </div>
    </div>
  )
}
