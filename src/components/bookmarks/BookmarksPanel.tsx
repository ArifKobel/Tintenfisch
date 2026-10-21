import type { KeyboardEvent } from 'react'
import { BookmarkRow } from '@/components/bookmarks/BookmarkRow.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { SidePanel } from '@/components/ui/SidePanel.tsx'
import { useBookmarksStore } from '@/stores/bookmarksStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function BookmarksPanel() {
  const bookmarks = useBookmarksStore()
  const workspace = useWorkspaceStore()
  const { items, selected, notice } = bookmarks

  function pressed(event: KeyboardEvent<HTMLElement>) {
    if (!selected || event.target instanceof HTMLInputElement) return
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault()
      void bookmarks.removeSelected()
    } else if (event.key === 'F2') {
      bookmarks.startRenaming(selected)
    }
  }

  const actions = (
    <div className="panel-actions">
      <IconButton icon="add" label="Add bookmark" onClick={() => void bookmarks.add()} />
      <IconButton icon="target" label="Set to current view" disabled={!selected} onClick={() => void bookmarks.setDestination()} />
      <IconButton icon="delete" label="Delete bookmark" disabled={!selected} onClick={() => void bookmarks.removeSelected()} />
    </div>
  )

  return (
    <SidePanel title="Bookmarks" side="right" onClose={() => workspace.showRight(null)} actions={actions} onKeyDown={pressed}>
      {items.length === 0 ? (
        <div className="empty-state">
          <h3 className="empty-state-title">No bookmarks yet</h3>
          <p>Add one to mark the page you are looking at.</p>
        </div>
      ) : (
        <ul className="bookmark-tree" role="tree">
          {items.map((bookmark, index) => (
            <BookmarkRow key={bookmark.id} bookmark={bookmark} parent={null} index={index} depth={0} />
          ))}
        </ul>
      )}
      {notice && (
        <p className="notice" role="alert">
          {notice}
        </p>
      )}
    </SidePanel>
  )
}
