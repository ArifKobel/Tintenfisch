import { useCommentsStore } from '@/stores/commentsStore.ts'
import { useRedactionStore } from '@/stores/redactionStore.ts'

export function RedactionControl() {
  const redaction = useRedactionStore()
  const { comments } = useCommentsStore()
  const { confirming, applying, notice } = redaction
  const marks = comments.filter((comment) => comment.subtype === 'Redact').length
  if (marks === 0 && !notice) return null
  return (
    <div className="context-group" role="group" aria-label="Redaction">
      {confirming ? (
        <>
          <span className="context-text">
            Remove everything under {marks === 1 ? 'the mark' : `${marks} marks`} for good?
          </span>
          <button type="button" className="button button-danger" onClick={() => void redaction.apply()}>
            Apply
          </button>
          <button type="button" className="button" onClick={() => redaction.cancel()}>
            Cancel
          </button>
        </>
      ) : (
        marks > 0 && (
          <button type="button" className="button" disabled={applying} onClick={() => redaction.requestApply()}>
            {applying ? 'Applying redactions' : `Apply redactions (${marks})`}
          </button>
        )
      )}
      {notice && (
        <span className="notice" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
