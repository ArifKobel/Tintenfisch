import { useApplication } from '@app/ApplicationContext.ts'
import { useCommentsState } from '@comments/useCommentsState.ts'
import { useDocumentState } from '@document/useDocumentState.ts'

export function CommentsButton() {
  const { comments } = useApplication()
  const { comments: all, open } = useCommentsState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  return (
    <button type="button" className={`button${open ? ' button-active' : ''}`} aria-pressed={open} onClick={() => comments.togglePanel()}>
      Comments ({all.length})
    </button>
  )
}
