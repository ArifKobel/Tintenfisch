import { BookmarksPanel } from '@/components/bookmarks/BookmarksPanel.tsx'
import { CommentsPanel } from '@/components/comments/CommentsPanel.tsx'
import { FieldPanel } from '@/components/forms/FieldPanel.tsx'
import { PagesPanel } from '@/components/pages/PagesPanel.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function RightDock() {
  const right = useWorkspaceStore((state) => state.right)
  const phase = useDocumentStore((state) => state.document.phase)
  if (phase !== 'ready') return null
  return (
    <>
      {right === 'comments' && <CommentsPanel />}
      {right === 'pages' && <PagesPanel />}
      {right === 'bookmarks' && <BookmarksPanel />}
      <FieldPanel />
    </>
  )
}
