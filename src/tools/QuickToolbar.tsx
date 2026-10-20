import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { usePanelState } from '@shell/usePanelState.ts'
import { SignMenuButton } from '@signing/SignMenuButton.tsx'
import { ToolGroupButton } from '@tools/ToolGroupButton.tsx'
import type { Tool } from '@tools/ToolStore.ts'
import { useToolState } from '@tools/useToolState.ts'
import { IconButton } from '@ui/IconButton.tsx'

const GROUPS: { label: string; tools: readonly Tool[] }[] = [
  { label: 'Select', tools: ['select'] },
  { label: 'Edit text and images', tools: ['edit'] },
  { label: 'Add a comment', tools: ['note', 'textBox', 'stamp'] },
  { label: 'Highlight text', tools: ['highlight'] },
  { label: 'Draw', tools: ['ink', 'rectangle', 'ellipse', 'arrow'] },
  { label: 'Prepare form', tools: ['form'] },
]

export function QuickToolbar() {
  const { panels } = useApplication()
  const { tool } = useToolState()
  const { left, sheet } = usePanelState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
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
            onClick={() => (sheet ? panels.closeSheet() : panels.openSheet())}
          />
        </span>
      </div>
    </div>
  )
}
