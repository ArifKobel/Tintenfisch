import { CommentsPanel } from '@comments/CommentsPanel.tsx'
import { useDocumentState } from '@document/useDocumentState.ts'
import { FieldPanel } from '@forms/FieldPanel.tsx'
import { BookmarksPanel } from '@navigation/BookmarksPanel.tsx'
import { usePanelState } from '@shell/usePanelState.ts'
import { PagesPanel } from '@thumbnails/PagesPanel.tsx'

export function RightPanels() {
  const { right } = usePanelState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  return (
    <>
      {right === 'comments' && <CommentsPanel />}
      {right === 'pages' && <PagesPanel />}
      {right === 'bookmarks' && <BookmarksPanel />}
      <FieldPanel />
    </>
  )
}
