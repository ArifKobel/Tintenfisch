import { useApplication } from '@app/ApplicationContext.ts'
import { useCommentsState } from '@comments/useCommentsState.ts'
import { useRedactionState } from '@redaction/useRedactionState.ts'

export function RedactionControl() {
  const { redaction } = useApplication()
  const { comments } = useCommentsState()
  const { confirming, applying, notice } = useRedactionState()
  const marks = comments.filter((comment) => comment.subtype === 'Redact').length
  if (marks === 0 && !notice) return null
  return (
    <div className="redaction-control" role="group" aria-label="Redaction">
      {confirming ? (
        <>
          <span className="redaction-warning">
            Remove everything under {marks === 1 ? 'the mark' : `${marks} marks`} for good?
          </span>
          <button type="button" className="button button-small button-danger" onClick={() => void redaction.apply()}>
            Apply
          </button>
          <button type="button" className="button button-small" onClick={() => redaction.cancel()}>
            Cancel
          </button>
        </>
      ) : (
        marks > 0 && (
          <button type="button" className="button button-small" disabled={applying} onClick={() => redaction.requestApply()}>
            {applying ? 'Applying redactions' : `Apply redactions (${marks})`}
          </button>
        )
      )}
      {notice && (
        <span className="edit-notice" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
