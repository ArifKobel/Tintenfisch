import type { MouseEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { Comment } from '@engine/protocol.ts'

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
  const { comments, viewport } = useApplication()

  function reveal() {
    comments.select(comment.id)
    viewport.focusPage(comment.pageId, comment.rect.y)
  }

  function remove(event: MouseEvent) {
    event.stopPropagation()
    void comments.remove(comment.id)
  }

  return (
    <article className={`comment-item${selected ? ' comment-item-selected' : ''}`} onClick={reveal}>
      <header className="comment-item-header">
        <span className="comment-kind">{KINDS[comment.subtype] ?? comment.subtype}</span>
        <span className="comment-page">Page {pageNumber}</span>
        <button type="button" className="button button-small" onClick={remove}>
          Delete
        </button>
      </header>
      <textarea
        key={comment.contents}
        className="comment-text"
        defaultValue={comment.contents}
        placeholder="Add a comment"
        aria-label="Comment"
        onBlur={(event) => void comments.setContents(comment.id, event.target.value)}
      />
      {(comment.author || comment.date) && (
        <footer className="comment-meta">{[comment.author, readableDate(comment.date)].filter(Boolean).join(' · ')}</footer>
      )}
    </article>
  )
}
