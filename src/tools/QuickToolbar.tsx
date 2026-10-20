import { useDocumentState } from '@document/useDocumentState.ts'
import { SignMenuButton } from '@signing/SignMenuButton.tsx'
import { ToolGroupButton } from '@tools/ToolGroupButton.tsx'
import type { Tool } from '@tools/ToolStore.ts'
import { useToolState } from '@tools/useToolState.ts'

const GROUPS: { label: string; tools: readonly Tool[] }[] = [
  { label: 'Select', tools: ['select'] },
  { label: 'Edit text and images', tools: ['edit'] },
  { label: 'Add a comment', tools: ['note', 'textBox', 'stamp'] },
  { label: 'Highlight text', tools: ['highlight'] },
  { label: 'Draw', tools: ['ink', 'rectangle', 'ellipse', 'arrow'] },
  { label: 'Prepare form', tools: ['form'] },
]

export function QuickToolbar() {
  const { tool } = useToolState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  return (
    <div className="quick-toolbar-gutter">
      <div className="quick-toolbar" role="toolbar" aria-label="Quick tools" aria-orientation="vertical">
        {GROUPS.map((group) => (
          <ToolGroupButton key={group.label} label={group.label} tools={group.tools} current={tool} />
        ))}
        <SignMenuButton />
      </div>
    </div>
  )
}
