import { CommentItem } from '@comments/CommentItem.tsx'
import { useCommentsState } from '@comments/useCommentsState.ts'
import { useDocumentState } from '@document/useDocumentState.ts'

export function CommentsPanel() {
  const { comments, selected, open, notice } = useCommentsState()
  const document = useDocumentState()
  if (!open || document.phase !== 'ready') return null
  const numbers = new Map(document.pages.map((page, index) => [page.id, index + 1]))
  return (
    <aside className="comments-panel" aria-label="Comments">
      <h2 className="comments-title">Comments</h2>
      {notice && (
        <p className="comments-notice" role="status">
          {notice}
        </p>
      )}
      {comments.length === 0 && <p className="comments-empty">No comments yet.</p>}
      {comments.map((comment) => (
        <CommentItem key={comment.id} comment={comment} pageNumber={numbers.get(comment.pageId) ?? 0} selected={comment.id === selected} />
      ))}
    </aside>
  )
}
