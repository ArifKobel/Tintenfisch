import { useApplication } from '@app/ApplicationContext.ts'
import { useLinksState } from '@navigation/useLinksState.ts'
import { useToolState } from '@tools/useToolState.ts'

export function LinkControls() {
  const { links } = useApplication()
  const { selected, notice } = useLinksState()
  const { tool } = useToolState()
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
