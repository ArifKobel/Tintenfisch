import { useCommentsStore } from '@/stores/commentsStore.ts'
import { useRedactionStore } from '@/stores/redactionStore.ts'
import { Button } from '@/components/ui/Button.tsx'

export function RedactionControl() {
  const redaction = useRedactionStore()
  const { comments } = useCommentsStore()
  const { confirming, applying, notice } = redaction
  const marks = comments.filter((comment) => comment.subtype === 'Redact').length
  if (marks === 0 && !notice) return null
  return (
    <div className="flex min-h-10 items-center gap-2" role="group" aria-label="Redaction">
      {confirming ? (
        <>
          <span className="text-ink-strong">
            Remove everything under {marks === 1 ? 'the mark' : `${marks} marks`} for good?
          </span>
          <Button look="danger" onClick={() => void redaction.apply()}>
            Apply
          </Button>
          <Button onClick={() => redaction.cancel()}>
            Cancel
          </Button>
        </>
      ) : (
        marks > 0 && (
          <Button disabled={applying} onClick={() => redaction.requestApply()}>
            {applying ? 'Applying redactions' : `Apply redactions (${marks})`}
          </Button>
        )
      )}
      {notice && (
        <span className="text-[13px] text-negative" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
