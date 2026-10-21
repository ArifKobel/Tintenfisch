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
    <div className="relative z-12 w-[66px] flex-none max-md:order-1 max-md:w-auto max-md:border-t max-md:border-divider max-md:bg-surface">
      <div className="absolute top-4 left-4 flex w-10 flex-col items-center gap-2 rounded bg-surface py-1 shadow-float max-md:static max-md:w-auto max-md:flex-row max-md:justify-between max-md:gap-0 max-md:rounded-none max-md:px-3 max-md:pt-2 max-md:pb-[calc(8px+env(safe-area-inset-bottom))] max-md:shadow-none max-md:[&>:nth-child(n+5)_[role=menu]]:right-0 max-md:[&>:nth-child(n+5)_[role=menu]]:left-auto" role="toolbar" aria-label="Quick tools" aria-orientation="vertical">
        {GROUPS.map((group) => (
          <ToolGroupButton key={group.label} label={group.label} tools={group.tools} current={tool} />
        ))}
        <SignMenuButton />
        <span className="hidden max-md:contents">
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
