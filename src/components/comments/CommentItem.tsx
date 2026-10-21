import type { MouseEvent } from 'react'
import type { Comment } from '@/types/protocol.ts'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { useCommentsStore } from '@/stores/commentsStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'
import { TextArea } from '@/components/ui/Field.tsx'

const KINDS: Record<string, string> = {
  Highlight: 'Highlight',
  Text: 'Note',
  Ink: 'Drawing',
  Square: 'Rectangle',
  Circle: 'Ellipse',
  Line: 'Arrow',
  Stamp: 'Stamp',
  FreeText: 'Text box',
  Redact: 'Redaction',
}

function readableDate(date: string): string {
  const parts = /^D:(\d{4})(\d{2})(\d{2})(\d{2})?(\d{2})?/.exec(date)
  if (!parts) return ''
  const [, year, month, day, hour = '00', minute = '00'] = parts
  return `${year}-${month}-${day} ${hour}:${minute}`
}

export function CommentItem({ comment, pageNumber, selected }: { comment: Comment; pageNumber: number; selected: boolean }) {
  const comments = useCommentsStore()
  const viewport = useViewportStore()

  function reveal() {
    comments.select(comment.id)
    viewport.focusPage(comment.pageId, comment.rect.y)
  }

  function remove(event: MouseEvent) {
    event.stopPropagation()
    void comments.remove(comment.id)
  }

  return (
    <article className={`flex cursor-pointer flex-col gap-2 rounded-lg border bg-surface p-3 ${selected ? 'border-selection shadow-[0_0_0_1px_var(--color-selection)]' : 'border-separator hover:border-menu-border'}`} onClick={reveal}>
      <header className="flex items-center gap-2">
        <span className="font-bold text-ink-strong">{KINDS[comment.subtype] ?? comment.subtype}</span>
        <span className="flex-1 text-xs text-ink-muted">Page {pageNumber}</span>
        <IconButton icon="delete" label="Delete comment" tooltip="left" onClick={remove} />
      </header>
      <TextArea
        key={comment.contents}
        className="min-h-14 resize-y"
        defaultValue={comment.contents}
        placeholder="Add a comment"
        aria-label="Comment"
        onBlur={(event) => void comments.setContents(comment.id, event.target.value)}
      />
      {(comment.author || comment.date) && (
        <footer className="text-xs text-ink-muted">{[comment.author, readableDate(comment.date)].filter(Boolean).join(' · ')}</footer>
      )}
    </article>
  )
}
