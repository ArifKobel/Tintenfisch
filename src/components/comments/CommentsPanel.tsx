import { CommentItem } from '@/components/comments/CommentItem.tsx'
import { SidePanel } from '@/components/ui/SidePanel.tsx'
import { useCommentsStore } from '@/stores/commentsStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function CommentsPanel() {
  const workspace = useWorkspaceStore()
  const { comments, selected, notice } = useCommentsStore()
  const document = useDocumentStore((state) => state.document)
  if (document.phase !== 'ready') return null
  const numbers = new Map(document.pages.map((page, index) => [page.id, index + 1]))
  return (
    <SidePanel title="Comments" side="right" onClose={() => workspace.showRight(null)}>
      {notice && (
        <p className="notice" role="status">
          {notice}
        </p>
      )}
      {comments.length === 0 ? (
        <div className="empty-state">
          <h3 className="empty-state-title">Get started</h3>
          <p>Add comments to make notes for yourself or others. Pick a sticky note, a text box or a drawing tool on the left.</p>
          <button type="button" className="button" onClick={() => workspace.choose('note')}>
            Add a sticky note
          </button>
        </div>
      ) : (
        <div className="comment-list">
          {comments.map((comment) => (
            <CommentItem key={comment.id} comment={comment} pageNumber={numbers.get(comment.pageId) ?? 0} selected={comment.id === selected} />
          ))}
        </div>
      )}
    </SidePanel>
  )
}
