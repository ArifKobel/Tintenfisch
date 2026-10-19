import { useApplication } from '@app/ApplicationContext.ts'
import { CommentItem } from '@comments/CommentItem.tsx'
import { useCommentsState } from '@comments/useCommentsState.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { SidePanel } from '@ui/SidePanel.tsx'

export function CommentsPanel() {
  const { panels, tools } = useApplication()
  const { comments, selected, notice } = useCommentsState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  const numbers = new Map(document.pages.map((page, index) => [page.id, index + 1]))
  return (
    <SidePanel title="Comments" side="right" onClose={() => panels.showRight(null)}>
      {notice && (
        <p className="notice" role="status">
          {notice}
        </p>
      )}
      {comments.length === 0 ? (
        <div className="empty-state">
          <h3 className="empty-state-title">Get started</h3>
          <p>Add comments to make notes for yourself or others. Pick a sticky note, a text box or a drawing tool on the left.</p>
          <button type="button" className="button" onClick={() => tools.choose('note')}>
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
