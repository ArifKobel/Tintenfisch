import { useState, type DragEvent, type KeyboardEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { Bookmark } from '@engine/protocol.ts'
import type { BookmarkPlace } from '@navigation/BookmarksStore.ts'
import { useBookmarksState } from '@navigation/useBookmarksState.ts'

export const BOOKMARK_TYPE = 'application/x-tintenfisch-bookmark'

type Zone = 'before' | 'inside' | 'after'

function zoneOf(event: DragEvent<HTMLElement>): Zone {
  const bounds = event.currentTarget.getBoundingClientRect()
  const share = (event.clientY - bounds.top) / Math.max(1, bounds.height)
  return share < 0.25 ? 'before' : share > 0.75 ? 'after' : 'inside'
}

export function BookmarkRow({ bookmark, parent, index, depth }: { bookmark: Bookmark; parent: string | null; index: number; depth: number }) {
  const { bookmarks } = useApplication()
  const { selected, renaming } = useBookmarksState()
  const [zone, setZone] = useState<Zone | null>(null)
  const collapsed = bookmarks.isCollapsed(bookmark)
  const hasChildren = bookmark.children.length > 0

  function dropped(event: DragEvent<HTMLElement>) {
    event.preventDefault()
    setZone(null)
    const id = event.dataTransfer.getData(BOOKMARK_TYPE)
    if (!id || id === bookmark.id) return
    const places: Record<Zone, BookmarkPlace> = {
      before: { parent, index },
      after: { parent, index: index + 1 },
      inside: { parent: bookmark.id, index: bookmark.children.length },
    }
    void bookmarks.move(id, places[zoneOf(event)])
  }

  function typed(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') event.currentTarget.blur()
    else if (event.key === 'Escape') bookmarks.stopRenaming()
    event.stopPropagation()
  }

  return (
    <li className="bookmark" role="treeitem" aria-expanded={hasChildren ? !collapsed : undefined} aria-selected={selected === bookmark.id}>
      <div
        className={`bookmark-row${selected === bookmark.id ? ' bookmark-row-selected' : ''}${zone ? ` bookmark-drop-${zone}` : ''}`}
        style={{ paddingLeft: 4 + depth * 14 }}
        draggable={renaming !== bookmark.id}
        onDragStart={(event) => {
          event.dataTransfer.setData(BOOKMARK_TYPE, bookmark.id)
          event.dataTransfer.effectAllowed = 'move'
        }}
        onDragOver={(event) => {
          if (!event.dataTransfer.types.includes(BOOKMARK_TYPE)) return
          event.preventDefault()
          setZone(zoneOf(event))
        }}
        onDragLeave={() => setZone(null)}
        onDrop={dropped}
      >
        <button
          type="button"
          className={`bookmark-toggle${hasChildren ? '' : ' bookmark-toggle-empty'}${collapsed ? '' : ' bookmark-toggle-open'}`}
          aria-label={collapsed ? 'Expand' : 'Collapse'}
          tabIndex={hasChildren ? 0 : -1}
          onClick={() => hasChildren && bookmarks.toggleCollapsed(bookmark)}
        />
        {renaming === bookmark.id ? (
          <input
            className="bookmark-rename"
            aria-label="Bookmark title"
            defaultValue={bookmark.title}
            autoFocus
            onFocus={(event) => event.currentTarget.select()}
            onBlur={(event) => void bookmarks.rename(bookmark.id, event.currentTarget.value)}
            onKeyDown={typed}
          />
        ) : (
          <button type="button" className="bookmark-title" onClick={() => bookmarks.follow(bookmark)} onDoubleClick={() => bookmarks.startRenaming(bookmark.id)}>
            {bookmark.title || 'Untitled'}
          </button>
        )}
      </div>
      {hasChildren && !collapsed && (
        <ul className="bookmark-children" role="group">
          {bookmark.children.map((child, childIndex) => (
            <BookmarkRow key={child.id} bookmark={child} parent={bookmark.id} index={childIndex} depth={depth + 1} />
          ))}
        </ul>
      )}
    </li>
  )
}
