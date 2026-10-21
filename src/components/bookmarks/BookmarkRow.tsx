import { useState, type DragEvent, type KeyboardEvent } from 'react'
import type { Bookmark } from '@/types/protocol.ts'
import { useBookmarksStore, type BookmarkPlace } from '@/stores/bookmarksStore.ts'
import { Input } from '@/components/ui/Field.tsx'

export const BOOKMARK_TYPE = 'application/x-tintenfisch-bookmark'

type Zone = 'before' | 'inside' | 'after'

function zoneOf(event: DragEvent<HTMLElement>): Zone {
  const bounds = event.currentTarget.getBoundingClientRect()
  const share = (event.clientY - bounds.top) / Math.max(1, bounds.height)
  return share < 0.25 ? 'before' : share > 0.75 ? 'after' : 'inside'
}


const DROP_ZONES = {
  before: 'shadow-[inset_0_2px_0_var(--color-selection)]',
  after: 'shadow-[inset_0_-2px_0_var(--color-selection)]',
  inside: 'outline-1 outline-selection',
}

export function BookmarkRow({ bookmark, parent, index, depth }: { bookmark: Bookmark; parent: string | null; index: number; depth: number }) {
  const bookmarks = useBookmarksStore()
  const { selected, renaming } = bookmarks
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
    <li role="treeitem" aria-expanded={hasChildren ? !collapsed : undefined} aria-selected={selected === bookmark.id}>
      <div
        className={`flex min-h-8 items-center gap-0.5 rounded ${selected === bookmark.id ? 'bg-selection-fill' : 'hover:bg-hover'} ${zone ? DROP_ZONES[zone] : ''}`}
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
          className={`size-5 flex-none cursor-pointer p-0 before:mx-auto before:block before:size-1.5 before:border-r-[1.5px] before:border-b-[1.5px] before:border-ink before:content-[''] ${collapsed ? 'before:-rotate-45' : 'before:rotate-45'} ${hasChildren ? '' : 'invisible'}`}
          aria-label={collapsed ? 'Expand' : 'Collapse'}
          tabIndex={hasChildren ? 0 : -1}
          onClick={() => hasChildren && bookmarks.toggleCollapsed(bookmark)}
        />
        {renaming === bookmark.id ? (
          <Input
            className="min-w-0 flex-1"
            aria-label="Bookmark title"
            defaultValue={bookmark.title}
            autoFocus
            onFocus={(event) => event.currentTarget.select()}
            onBlur={(event) => void bookmarks.rename(bookmark.id, event.currentTarget.value)}
            onKeyDown={typed}
          />
        ) : (
          <button type="button" className="flex-1 cursor-pointer truncate p-1 text-left text-ink-strong" onClick={() => bookmarks.follow(bookmark)} onDoubleClick={() => bookmarks.startRenaming(bookmark.id)}>
            {bookmark.title || 'Untitled'}
          </button>
        )}
      </div>
      {hasChildren && !collapsed && (
        <ul className="m-0 list-none p-0" role="group">
          {bookmark.children.map((child, childIndex) => (
            <BookmarkRow key={child.id} bookmark={child} parent={bookmark.id} index={childIndex} depth={depth + 1} />
          ))}
        </ul>
      )}
    </li>
  )
}
