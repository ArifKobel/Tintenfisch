import { CommentsPanel } from '@/components/comments/CommentsPanel.tsx'
import { useDocumentState } from '@/hooks/useDocumentState.ts'
import { FieldPanel } from '@/components/forms/FieldPanel.tsx'
import { BookmarksPanel } from '@/components/bookmarks/BookmarksPanel.tsx'
import { usePanelState } from '@/hooks/usePanelState.ts'
import { PagesPanel } from '@/components/pages/PagesPanel.tsx'

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
