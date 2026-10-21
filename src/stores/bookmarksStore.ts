import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentChanged, onDocumentReplaced } from '@/stores/documentStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'
import type { Bookmark, Command } from '@/types/protocol.ts'

export const UNTITLED = 'Untitled'

export interface BookmarkPlace {
  parent: string | null
  index: number
}

interface BookmarksStore {
  items: Bookmark[]
  selected: string | null
  renaming: string | null
  collapsed: Readonly<Record<string, boolean>>
  notice: string | null
  reload(): Promise<void>
  select(id: string | null): void
  isCollapsed(bookmark: Bookmark): boolean
  toggleCollapsed(bookmark: Bookmark): void
  follow(bookmark: Bookmark): void
  startRenaming(id: string): void
  stopRenaming(): void
  add(): Promise<void>
  rename(id: string, title: string): Promise<void>
  setDestination(): Promise<void>
  move(id: string, place: BookmarkPlace): Promise<void>
  removeSelected(): Promise<void>
  placeOf(id: string): BookmarkPlace | null
  reset(): void
}

function placeIn(items: Bookmark[], id: string, parent: string | null = null): BookmarkPlace | null {
  for (const [index, item] of items.entries()) {
    if (item.id === id) return { parent, index }
    const inside = placeIn(item.children, id, item.id)
    if (inside) return inside
  }
  return null
}

function findIn(items: Bookmark[], id: string): Bookmark | null {
  for (const item of items) {
    if (item.id === id) return item
    const inside = findIn(item.children, id)
    if (inside) return inside
  }
  return null
}

export const useBookmarksStore = create<BookmarksStore>()((set, get) => {
  const attempt = async (work: () => Promise<void>) => {
    try {
      await work()
      if (get().notice) set({ notice: null })
    } catch (error) {
      if (!(error instanceof CancelledError)) set({ notice: (error as Error).message })
    }
  }

  const change = (command: Command) =>
    attempt(async () => {
      await engine.request(command)
      await get().reload()
    })

  return {
    items: [],
    selected: null,
    renaming: null,
    collapsed: {},
    notice: null,

    async reload() {
      await attempt(async () => {
        const items = await engine.request({ kind: 'outline' }, { group: 'outline' })
        const known = (id: string | null) => (id && placeIn(items, id) ? id : null)
        set({ items, selected: known(get().selected), renaming: known(get().renaming) })
      })
    },

    select(id) {
      if (id !== get().selected) set({ selected: id, renaming: null })
    },

    isCollapsed(bookmark) {
      return get().collapsed[bookmark.id] ?? !bookmark.open
    },

    toggleCollapsed(bookmark) {
      set({ collapsed: { ...get().collapsed, [bookmark.id]: !get().isCollapsed(bookmark) } })
    },

    follow(bookmark) {
      get().select(bookmark.id)
      useViewportStore.getState().reveal(bookmark.target)
    },

    startRenaming(id) {
      set({ selected: id, renaming: id })
    },

    stopRenaming() {
      if (get().renaming) set({ renaming: null })
    },

    async add() {
      const { items, selected } = get()
      const current = selected ? placeIn(items, selected) : null
      const place = current ? { parent: current.parent, index: current.index + 1 } : { parent: null, index: items.length }
      await attempt(async () => {
        const id = await engine.request({ kind: 'addBookmark', ...place, title: UNTITLED, target: useViewportStore.getState().currentView() })
        await get().reload()
        get().startRenaming(id)
      })
    },

    async rename(id, title) {
      get().stopRenaming()
      const trimmed = title.trim()
      if (trimmed === '' || trimmed === findIn(get().items, id)?.title) return
      await change({ kind: 'renameBookmark', bookmarkId: id, title: trimmed })
    },

    async setDestination() {
      const target = useViewportStore.getState().currentView()
      const id = get().selected
      if (id && target) await change({ kind: 'retargetBookmark', bookmarkId: id, target })
    },

    async move(id, place) {
      const current = get().placeOf(id)
      if (!current || id === place.parent) return
      const shifted = current.parent === place.parent && current.index < place.index
      const index = shifted ? place.index - 1 : place.index
      if (current.parent === place.parent && current.index === index) return
      if (place.parent) set({ collapsed: { ...get().collapsed, [place.parent]: false } })
      await change({ kind: 'moveBookmark', bookmarkId: id, parent: place.parent, index })
    },

    async removeSelected() {
      const id = get().selected
      if (id) await change({ kind: 'deleteBookmark', bookmarkId: id })
    },

    placeOf: (id) => placeIn(get().items, id),

    reset() {
      set({ items: [], selected: null, renaming: null, collapsed: {}, notice: null })
    },
  }
})

onDocumentReplaced((ready) => {
  useBookmarksStore.getState().reset()
  if (ready) void useBookmarksStore.getState().reload()
})

onDocumentChanged(() => void useBookmarksStore.getState().reload())
