import type { KeyboardEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { BookmarkRow } from '@navigation/BookmarkRow.tsx'
import { useBookmarksState } from '@navigation/useBookmarksState.ts'

export function BookmarksPanel() {
  const { bookmarks } = useApplication()
  const { open, items, selected, notice } = useBookmarksState()
  if (!open) return null

  function pressed(event: KeyboardEvent<HTMLElement>) {
    if (!selected || event.target instanceof HTMLInputElement) return
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault()
      void bookmarks.removeSelected()
    } else if (event.key === 'F2') {
      bookmarks.startRenaming(selected)
    }
  }

  return (
    <aside className="bookmarks-panel" aria-label="Bookmarks" onKeyDown={pressed}>
      <div className="bookmarks-header">
        <h2 className="bookmarks-title">Bookmarks</h2>
        <button type="button" className="button button-small" onClick={() => void bookmarks.add()}>
          Add
        </button>
        <button type="button" className="button button-small" disabled={!selected} onClick={() => void bookmarks.setDestination()}>
          Set to view
        </button>
        <button type="button" className="button button-small" disabled={!selected} onClick={() => void bookmarks.removeSelected()}>
          Delete
        </button>
      </div>
      {items.length === 0 ? (
        <p className="bookmarks-empty">No bookmarks yet. Add one to mark the page you are looking at.</p>
      ) : (
        <ul className="bookmark-tree" role="tree">
          {items.map((bookmark, index) => (
            <BookmarkRow key={bookmark.id} bookmark={bookmark} parent={null} index={index} depth={0} />
          ))}
        </ul>
      )}
      {notice && (
        <p className="edit-notice" role="alert">
          {notice}
        </p>
      )}
    </aside>
  )
}
