import { create } from 'zustand'
import { engine } from '@/services/EngineClient.ts'
import type { Opened, OpenResult, PageSummary } from '@/types/protocol.ts'

export type ReadyDocument = { phase: 'ready'; name: string; session: number; pages: PageSummary[] } & Opened

export type DocumentState =
  | { phase: 'empty' }
  | { phase: 'loading'; name: string }
  | { phase: 'locked'; name: string; wrongPassword: boolean }
  | ReadyDocument
  | { phase: 'failed'; name: string; message: string }

interface DocumentStore {
  document: DocumentState
  sessions: number
  pageRevisions: Readonly<Record<string, number>>
  changes: number
  open(name: string, bytes: Uint8Array): Promise<void>
  unlock(password: string): Promise<void>
  close(): void
  replacePages(pages: PageSummary[]): void
  refreshPages(): Promise<void>
  save(optimize?: boolean): Promise<Uint8Array>
  touchPages(pageIds: Iterable<string>): void
  markChanged(): void
}

export const useDocumentStore = create<DocumentStore>()((set, get) => {
  const settle = async (name: string, opening: () => Promise<OpenResult>) => {
    try {
      const result = await opening()
      if (result.status === 'locked') {
        set({ document: { phase: 'locked', name, wrongPassword: result.wrongPassword } })
        return
      }
      const { status: _status, ...opened } = result
      const pages = await engine.request({ kind: 'pages' })
      const session = get().sessions + 1
      set({ sessions: session, document: { phase: 'ready', name, session, pages, ...opened } })
    } catch (error) {
      set({ document: { phase: 'failed', name, message: (error as Error).message } })
    }
  }

  return {
    document: { phase: 'empty' },
    sessions: 0,
    pageRevisions: {},
    changes: 0,

    async open(name, bytes) {
      set({ document: { phase: 'loading', name } })
      await settle(name, () => engine.request({ kind: 'open', bytes }))
    },

    async unlock(password) {
      const { document } = get()
      if (document.phase !== 'locked') return
      set({ document: { phase: 'loading', name: document.name } })
      await settle(document.name, () => engine.request({ kind: 'unlock', password }))
    },

    close() {
      if (get().document.phase !== 'empty') set({ document: { phase: 'empty' } })
    },

    replacePages(pages) {
      const { document } = get()
      if (document.phase === 'ready') set({ document: { ...document, pages } })
    },

    async refreshPages() {
      if (get().document.phase !== 'ready') return
      get().replacePages(await engine.request({ kind: 'pages' }))
    },

    async save(optimize = false) {
      if (get().document.phase !== 'ready') throw new Error('no document is open')
      return engine.request({ kind: 'save', optimize })
    },

    touchPages(pageIds) {
      const pageRevisions = { ...get().pageRevisions }
      for (const pageId of pageIds) pageRevisions[pageId] = (pageRevisions[pageId] ?? 0) + 1
      set({ pageRevisions })
    },

    markChanged() {
      set({ changes: get().changes + 1 })
    },
  }
})

export function onDocumentReplaced(listener: (ready: boolean) => void): () => void {
  return useDocumentStore.subscribe(({ document }, previous) => {
    if (document === previous.document) return
    const before = previous.document
    if (document.phase === 'ready' && before.phase === 'ready' && document.session === before.session) return
    listener(document.phase === 'ready')
  })
}

export function onDocumentChanged(listener: () => void): () => void {
  return useDocumentStore.subscribe(({ changes }, previous) => {
    if (changes !== previous.changes) listener()
  })
}
