import { useLinksStore } from '@/stores/linksStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function LinkControls() {
  const links = useLinksStore()
  const { selected, notice } = links
  const { tool } = useWorkspaceStore()
  if (tool !== 'link') return null
  return (
    <div className="context-group" role="group" aria-label="Link">
      {selected ? (
        <>
          <button type="button" className="button" onClick={() => links.editSelected()}>
            Edit link
          </button>
          <button type="button" className="button" onClick={() => void links.removeSelected()}>
            Delete link
          </button>
        </>
      ) : (
        <span className="hint">Drag over the page to add a link</span>
      )}
      {notice && (
        <span className="notice" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
