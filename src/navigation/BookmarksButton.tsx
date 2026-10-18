import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { useBookmarksState } from '@navigation/useBookmarksState.ts'

export function BookmarksButton() {
  const { bookmarks } = useApplication()
  const { open } = useBookmarksState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  return (
    <button type="button" className={`button${open ? ' button-active' : ''}`} aria-pressed={open} onClick={() => bookmarks.togglePanel()}>
      Bookmarks
    </button>
  )
}
