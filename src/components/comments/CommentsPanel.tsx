import { CommentItem } from '@/components/comments/CommentItem.tsx'
import { EmptyState, SidePanel } from '@/components/ui/SidePanel.tsx'
import { useCommentsStore } from '@/stores/commentsStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Notice } from '@/components/ui/Text.tsx'

export function CommentsPanel() {
  const workspace = useWorkspaceStore()
  const { comments, selected, notice } = useCommentsStore()
  const document = useDocumentStore((state) => state.document)
  if (document.phase !== 'ready') return null
  const numbers = new Map(document.pages.map((page, index) => [page.id, index + 1]))
  return (
    <SidePanel title="Comments" side="right" onClose={() => workspace.showRight(null)}>
      {notice && (
        <Notice>{notice}</Notice>
      )}
      {comments.length === 0 ? (
        <EmptyState title="Get started">
          <p>Add comments to make notes for yourself or others. Pick a sticky note, a text box or a drawing tool on the left.</p>
          <Button onClick={() => workspace.choose('note')}>Add a sticky note</Button>
        </EmptyState>
      ) : (
        <div className="flex flex-col gap-3">
          {comments.map((comment) => (
            <CommentItem key={comment.id} comment={comment} pageNumber={numbers.get(comment.pageId) ?? 0} selected={comment.id === selected} />
          ))}
        </div>
      )}
    </SidePanel>
  )
}
