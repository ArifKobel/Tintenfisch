import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'
import { onToolChosen } from '@/stores/workspaceStore.ts'
import type { DocumentChange, Highlight } from '@/types/protocol.ts'

const BACKGROUND_SHARE = 0.5

export interface ImageChoice {
  pageId: string
  index: number
}

interface ImagesStore {
  pages: Readonly<Record<string, readonly Highlight[]>>
  selected: ImageChoice | null
  notice: string | null
  selectedFrame(): Highlight | null
  load(pageId: string): Promise<void>
  select(choice: ImageChoice | null): void
  selectAt(pageId: string, x: number, y: number): void
  foregroundAt(pageId: string, x: number, y: number, pageArea: number): number | null
  insert(pageId: string, bytes: Uint8Array, x: number, y: number): Promise<void>
  move(choice: ImageChoice, frame: Highlight): Promise<void>
  replaceSelected(bytes: Uint8Array): Promise<void>
  removeSelected(): Promise<void>
  reset(): void
}

function contains(frame: Highlight, x: number, y: number): boolean {
  const [dx, dy] = [x - frame.x, y - frame.y]
  const along = dx * Math.cos(frame.angle) + dy * Math.sin(frame.angle)
  const down = -dx * Math.sin(frame.angle) + dy * Math.cos(frame.angle)
  return along >= 0 && along <= frame.width && down >= 0 && down <= frame.height
}

export const useImagesStore = create<ImagesStore>()((set, get) => {
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

  const indexAt = (pageId: string, x: number, y: number, largestArea: number): number | null => {
    const frames = get().pages[pageId] ?? []
    const index = frames.findLastIndex((frame) => frame.width * frame.height < largestArea && contains(frame, x, y))
    return index < 0 ? null : index
  }

  return {
    pages: {},
    selected: null,
    notice: null,

    selectedFrame() {
      const { selected, pages } = get()
      return selected ? (pages[selected.pageId]?.[selected.index] ?? null) : null
    },

    async load(pageId) {
      await attempt(async () => {
        const frames = await engine.request({ kind: 'images', pageId }, { group: `images-${pageId}` })
        const { selected, pages } = get()
        const kept = selected?.pageId !== pageId || selected.index < frames.length ? selected : null
        set({ pages: { ...pages, [pageId]: frames }, selected: kept })
      })
    },

    select(choice) {
      const { selected } = get()
      if (choice?.pageId === selected?.pageId && choice?.index === selected?.index) return
      set({ selected: choice, notice: null })
    },

    selectAt(pageId, x, y) {
      const index = indexAt(pageId, x, y, Infinity)
      get().select(index === null ? null : { pageId, index })
    },

    foregroundAt: (pageId, x, y, pageArea) => indexAt(pageId, x, y, pageArea * BACKGROUND_SHARE),

    async insert(pageId, bytes, x, y) {
      await attempt(async () => {
        const { index, dirtyPages } = await engine.request({ kind: 'insertImage', pageId, bytes, x, y })
        await changed(pageId, { dirtyPages })
        get().select({ pageId, index })
      })
    },

    async move(choice, frame) {
      await attempt(async () => changed(choice.pageId, await engine.request({ kind: 'moveImage', ...choice, frame })))
    },

    async replaceSelected(bytes) {
      const choice = get().selected
      if (!choice) return
      await attempt(async () => changed(choice.pageId, await engine.request({ kind: 'replaceImage', ...choice, bytes })))
    },

    async removeSelected() {
      const choice = get().selected
      if (!choice) return
      await attempt(async () => {
        const change = await engine.request({ kind: 'deleteImage', ...choice })
        set({ selected: null })
        await changed(choice.pageId, change)
      })
    },

    reset() {
      set({ pages: {}, selected: null, notice: null })
    },
  }
})

onDocumentReplaced(() => useImagesStore.getState().reset())

onToolChosen((tool) => {
  if (tool !== 'edit') useImagesStore.getState().select(null)
})
