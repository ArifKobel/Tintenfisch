import { CancelledError, type EngineClient } from '@engine/EngineClient.ts'
import type { Bookmark, Command } from '@engine/protocol.ts'
import type { TargetFollower } from '@navigation/TargetFollower.ts'

export interface BookmarksState {
  items: Bookmark[]
  selected: string | null
  renaming: string | null
  collapsed: Readonly<Record<string, boolean>>
  notice: string | null
}

export interface BookmarkPlace {
  parent: string | null
  index: number
}

export const UNTITLED = 'Untitled'

type Listener = () => void

export class BookmarksStore {
  readonly #client: EngineClient
  readonly #follower: TargetFollower
  readonly #listeners = new Set<Listener>()
  #state: BookmarksState = { items: [], selected: null, renaming: null, collapsed: {}, notice: null }

  constructor(client: EngineClient, follower: TargetFollower) {
    this.#client = client
    this.#follower = follower
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): BookmarksState => this.#state

  async reload() {
    await this.#attempt(async () => {
      const items = await this.#client.request({ kind: 'outline' }, { group: 'outline' })
      const known = (id: string | null) => (id && BookmarksStore.#placeOf(items, id) ? id : null)
      this.#update({ ...this.#state, items, selected: known(this.#state.selected), renaming: known(this.#state.renaming) })
    })
  }

  select(id: string | null) {
    if (id !== this.#state.selected) this.#update({ ...this.#state, selected: id, renaming: null })
  }

  isCollapsed(bookmark: Bookmark): boolean {
    return this.#state.collapsed[bookmark.id] ?? !bookmark.open
  }

  toggleCollapsed(bookmark: Bookmark) {
    this.#update({ ...this.#state, collapsed: { ...this.#state.collapsed, [bookmark.id]: !this.isCollapsed(bookmark) } })
  }

  follow(bookmark: Bookmark) {
    this.select(bookmark.id)
    this.#follower.follow(bookmark.target)
  }

  startRenaming(id: string) {
    this.#update({ ...this.#state, selected: id, renaming: id })
  }

  stopRenaming() {
    if (this.#state.renaming) this.#update({ ...this.#state, renaming: null })
  }

  async add() {
    const selected = this.#state.selected ? BookmarksStore.#placeOf(this.#state.items, this.#state.selected) : null
    const place = selected ? { parent: selected.parent, index: selected.index + 1 } : { parent: null, index: this.#state.items.length }
    await this.#attempt(async () => {
      const id = await this.#client.request({ kind: 'addBookmark', ...place, title: UNTITLED, target: this.#follower.currentView() })
      await this.reload()
      this.startRenaming(id)
    })
  }

  async rename(id: string, title: string) {
    this.stopRenaming()
    const trimmed = title.trim()
    if (trimmed === '' || trimmed === BookmarksStore.#find(this.#state.items, id)?.title) return
    await this.#change({ kind: 'renameBookmark', bookmarkId: id, title: trimmed })
  }

  async setDestination() {
    const target = this.#follower.currentView()
    const id = this.#state.selected
    if (id && target) await this.#change({ kind: 'retargetBookmark', bookmarkId: id, target })
  }

  async move(id: string, place: BookmarkPlace) {
    const current = this.placeOf(id)
    if (!current || id === place.parent) return
    const shifted = current.parent === place.parent && current.index < place.index
    const index = shifted ? place.index - 1 : place.index
    if (current.parent === place.parent && current.index === index) return
    if (place.parent) this.#update({ ...this.#state, collapsed: { ...this.#state.collapsed, [place.parent]: false } })
    await this.#change({ kind: 'moveBookmark', bookmarkId: id, parent: place.parent, index })
  }

  async removeSelected() {
    const id = this.#state.selected
    if (id) await this.#change({ kind: 'deleteBookmark', bookmarkId: id })
  }

  placeOf(id: string): BookmarkPlace | null {
    return BookmarksStore.#placeOf(this.#state.items, id)
  }

  reset() {
    this.#update({ ...this.#state, items: [], selected: null, renaming: null, collapsed: {}, notice: null })
  }

  async #change(command: Command) {
    await this.#attempt(async () => {
      await this.#client.request(command)
      await this.reload()
    })
  }

  static #placeOf(items: Bookmark[], id: string, parent: string | null = null): BookmarkPlace | null {
    for (const [index, item] of items.entries()) {
      if (item.id === id) return { parent, index }
      const inside = BookmarksStore.#placeOf(item.children, id, item.id)
      if (inside) return inside
    }
    return null
  }

  static #find(items: Bookmark[], id: string): Bookmark | null {
    for (const item of items) {
      if (item.id === id) return item
      const inside = BookmarksStore.#find(item.children, id)
      if (inside) return inside
    }
    return null
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
      if (this.#state.notice) this.#update({ ...this.#state, notice: null })
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: BookmarksState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
