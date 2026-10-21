import { useLinksStore } from '@/stores/linksStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import { Button } from '@/components/ui/Button.tsx'

export function LinkControls() {
  const links = useLinksStore()
  const { selected, notice } = links
  const { tool } = useWorkspaceStore()
  if (tool !== 'link') return null
  return (
    <div className="flex min-h-10 items-center gap-2" role="group" aria-label="Link">
      {selected ? (
        <>
          <Button onClick={() => links.editSelected()}>
            Edit link
          </Button>
          <Button onClick={() => void links.removeSelected()}>
            Delete link
          </Button>
        </>
      ) : (
        <span className="text-[13px] text-ink-muted">Drag over the page to add a link</span>
      )}
      {notice && (
        <span className="text-[13px] text-negative" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
