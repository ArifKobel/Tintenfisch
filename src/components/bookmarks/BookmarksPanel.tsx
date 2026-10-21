import type { KeyboardEvent } from 'react'
import { BookmarkRow } from '@/components/bookmarks/BookmarkRow.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { EmptyState, PanelActions, SidePanel } from '@/components/ui/SidePanel.tsx'
import { useBookmarksStore } from '@/stores/bookmarksStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import { Notice } from '@/components/ui/Text.tsx'

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
    <PanelActions>
      <IconButton icon="add" label="Add bookmark" onClick={() => void bookmarks.add()} />
      <IconButton icon="target" label="Set to current view" disabled={!selected} onClick={() => void bookmarks.setDestination()} />
      <IconButton icon="delete" label="Delete bookmark" disabled={!selected} onClick={() => void bookmarks.removeSelected()} />
    </PanelActions>
  )

  return (
    <SidePanel title="Bookmarks" side="right" onClose={() => workspace.showRight(null)} actions={actions} onKeyDown={pressed}>
      {items.length === 0 ? (
        <EmptyState title="No bookmarks yet">
          <p>Add one to mark the page you are looking at.</p>
        </EmptyState>
      ) : (
        <ul className="m-0 list-none p-0" role="tree">
          {items.map((bookmark, index) => (
            <BookmarkRow key={bookmark.id} bookmark={bookmark} parent={null} index={index} depth={0} />
          ))}
        </ul>
      )}
      {notice && (
        <Notice role="alert">{notice}</Notice>
      )}
    </SidePanel>
  )
}
