import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'
import { onToolChosen } from '@/stores/workspaceStore.ts'
import type { Box, DocumentChange, DocumentLink, Target } from '@/types/protocol.ts'

export interface LinkChoice {
  pageId: string
  id: string
}

export interface LinkDraft {
  pageId: string
  linkId: string | null
  rect: Box
  target: Target | null
}

interface LinksStore {
  pages: Readonly<Record<string, readonly DocumentLink[]>>
  selected: LinkChoice | null
  draft: LinkDraft | null
  notice: string | null
  load(pageId: string): Promise<void>
  follow(link: DocumentLink): void
  select(choice: LinkChoice | null): void
  draw(pageId: string, rect: Box): void
  editSelected(): void
  closeDraft(): void
  save(target: Target): Promise<void>
  move(choice: LinkChoice, rect: Box): Promise<void>
  removeSelected(): Promise<void>
  reset(): void
}

export const useLinksStore = create<LinksStore>()((set, get) => {
  const attempt = async (work: () => Promise<void>) => {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) set({ notice: (error as Error).message })
    }
  }

  const changed = async (pageId: string, change: DocumentChange) => {
    useDocumentStore.getState().touchPages(change.dirtyPages)
    await get().load(pageId)
  }

  const linkOf = (choice: LinkChoice | null): DocumentLink | null =>
    (choice && get().pages[choice.pageId]?.find((link) => link.id === choice.id)) ?? null

  return {
    pages: {},
    selected: null,
    draft: null,
    notice: null,

    async load(pageId) {
      await attempt(async () => {
        const links = await engine.request({ kind: 'links', pageId }, { group: `links-${pageId}` })
        const { selected, pages } = get()
        const kept = selected?.pageId !== pageId || links.some((link) => link.id === selected.id) ? selected : null
        set({ pages: { ...pages, [pageId]: links }, selected: kept })
      })
    },

    follow(link) {
      useViewportStore.getState().reveal(link.target)
    },

    select(choice) {
      if (choice?.id !== get().selected?.id) set({ selected: choice, notice: null })
    },

    draw(pageId, rect) {
      set({ draft: { pageId, linkId: null, rect, target: null } })
    },

    editSelected() {
      const { selected } = get()
      const link = linkOf(selected)
      if (link && selected) set({ draft: { pageId: selected.pageId, linkId: link.id, rect: link.rect, target: link.target } })
    },

    closeDraft() {
      if (get().draft) set({ draft: null })
    },

    async save(target) {
      const { draft } = get()
      if (!draft) return
      await attempt(async () => {
        const { pageId, linkId, rect } = draft
        if (linkId) {
          await changed(pageId, await engine.request({ kind: 'updateLink', pageId, linkId, rect, target }))
          set({ draft: null })
        } else {
          const added = await engine.request({ kind: 'addLink', pageId, rect, target })
          await changed(pageId, added)
          set({ draft: null, selected: { pageId, id: added.id } })
        }
      })
    },

    async move(choice, rect) {
      const target = linkOf(choice)?.target
      if (!target) return
      await attempt(async () => changed(choice.pageId, await engine.request({ kind: 'updateLink', pageId: choice.pageId, linkId: choice.id, rect, target })))
    },

    async removeSelected() {
      const { selected } = get()
      if (!selected) return
      await attempt(async () => {
        const change = await engine.request({ kind: 'deleteComment', pageId: selected.pageId, commentId: selected.id })
        set({ selected: null, draft: null })
        await changed(selected.pageId, change)
      })
    },

    reset() {
      set({ pages: {}, selected: null, draft: null, notice: null })
    },
  }
})

onDocumentReplaced(() => useLinksStore.getState().reset())

onToolChosen((tool) => {
  if (tool !== 'link') useLinksStore.getState().select(null)
})
