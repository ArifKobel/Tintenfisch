import { create } from 'zustand'
import { PageSelection, type SelectionGesture } from '@/lib/PageSelection.ts'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'
import type { PagesChange, PageSummary } from '@/types/protocol.ts'

export interface ExtractedPages {
  name: string
  bytes: Uint8Array
}

interface PagesStore {
  selection: PageSelection
  notice: string | null
  lockedInsert: { name: string; wrongPassword: boolean } | null
  select(pageId: string, gesture: SelectionGesture): void
  rotate(quarterTurns: number): Promise<void>
  deleteSelected(): Promise<void>
  move(pageIds: string[], before: string | null): Promise<void>
  insertBlank(): Promise<void>
  insertFile(name: string, bytes: Uint8Array): Promise<void>
  unlockInsert(password: string): Promise<void>
  cancelInsert(): void
  extract(): Promise<ExtractedPages | null>
  reset(): void
}

function pages(): PageSummary[] {
  const { document } = useDocumentStore.getState()
  return document.phase === 'ready' ? document.pages : []
}

const order = () => pages().map((page) => page.id)

export const usePagesStore = create<PagesStore>()((set, get) => {
  let pendingInsert: { bytes: Uint8Array; before: string | null } | null = null

  const fail = (error: unknown) => {
    if (!(error instanceof CancelledError)) set({ notice: (error as Error).message })
  }

  const attempt = async (work: () => Promise<void>) => {
    try {
      await work()
    } catch (error) {
      fail(error)
    }
  }

  const selected = () => get().selection.inOrder(order())

  const insertionPoint = (): string | null => {
    const last = selected().at(-1)
    if (last === undefined) return null
    const pageIds = order()
    return pageIds[pageIds.indexOf(last) + 1] ?? null
  }

  const apply = (change: PagesChange) => {
    const documents = useDocumentStore.getState()
    documents.replacePages(change.pages)
    if (change.dirtyPages.length > 0) documents.touchPages(change.dirtyPages)
    set({ selection: get().selection.within(order()), notice: null })
    documents.markChanged()
  }

  const insert = async (name: string, password: string) => {
    const pending = pendingInsert
    if (!pending) return
    await attempt(async () => {
      const result = await engine.request({ kind: 'insertDocument', bytes: pending.bytes.slice(), password, before: pending.before })
      if (result.status === 'locked') {
        set({ lockedInsert: { name, wrongPassword: result.wrongPassword } })
        return
      }
      pendingInsert = null
      const documents = useDocumentStore.getState()
      documents.replacePages(result.pages)
      set({ selection: PageSelection.of(result.inserted), lockedInsert: null, notice: null })
      documents.markChanged()
    })
  }

  return {
    selection: PageSelection.EMPTY,
    notice: null,
    lockedInsert: null,

    select(pageId, gesture) {
      set({ selection: get().selection.click(pageId, order(), gesture) })
    },

    async rotate(quarterTurns) {
      const pageIds = selected()
      if (pageIds.length > 0) await attempt(async () => apply(await engine.request({ kind: 'rotatePages', pageIds, quarterTurns })))
    },

    async deleteSelected() {
      const pageIds = selected()
      if (pageIds.length > 0) await attempt(async () => apply(await engine.request({ kind: 'deletePages', pageIds })))
    },

    async move(pageIds, before) {
      if (pageIds.length > 0) await attempt(async () => apply(await engine.request({ kind: 'movePages', pageIds, before })))
    },

    async insertBlank() {
      const size = pages().find((page) => page.id === selected().at(-1)) ?? pages()[0]
      const before = insertionPoint()
      await attempt(async () => apply(await engine.request({ kind: 'insertBlankPage', before, width: size?.width ?? 595, height: size?.height ?? 842 })))
    },

    async insertFile(name, bytes) {
      pendingInsert = { bytes, before: insertionPoint() }
      await insert(name, '')
    },

    async unlockInsert(password) {
      const locked = get().lockedInsert
      if (locked) await insert(locked.name, password)
    },

    cancelInsert() {
      pendingInsert = null
      set({ lockedInsert: null })
    },

    async extract() {
      const pageIds = selected()
      const { document } = useDocumentStore.getState()
      if (pageIds.length === 0 || document.phase !== 'ready') return null
      try {
        const bytes = await engine.request({ kind: 'extractPages', pageIds })
        return { name: `${document.name.replace(/\.pdf$/i, '')} (pages).pdf`, bytes }
      } catch (error) {
        fail(error)
        return null
      }
    },

    reset() {
      pendingInsert = null
      set({ selection: PageSelection.EMPTY, notice: null, lockedInsert: null })
    },
  }
})

onDocumentReplaced(() => usePagesStore.getState().reset())
