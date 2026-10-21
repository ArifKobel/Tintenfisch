import { create } from 'zustand'
import { PageRange } from '@/lib/PageRange.ts'
import { CanvasPictureEncoder } from '@/services/CanvasPictureEncoder.ts'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useEditingStore } from '@/stores/editingStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'

const POINTS_PER_INCH = 72

export type PrintChoice = 'all' | 'current' | 'range'

export type PrintFit = 'fit' | 'actual'

export interface PrintSettings {
  choice: PrintChoice
  range: string
  fit: PrintFit
  resolution: number
}

export interface PrintedPage {
  url: string
  width: number
  height: number
}

interface PrintStore {
  dialogOpen: boolean
  settings: PrintSettings
  progress: { done: number; total: number } | null
  sheet: readonly PrintedPage[] | null
  notice: string | null
  openDialog(): void
  closeDialog(): void
  change(settings: Partial<PrintSettings>): void
  print(): Promise<void>
  finish(): void
}

function chosenPages({ choice, range }: PrintSettings, pageIds: string[]): number[] | null {
  if (choice === 'all') return pageIds.map((_, index) => index)
  if (choice === 'range') return PageRange.parse(range, pageIds.length)
  const current = pageIds.indexOf(useViewportStore.getState().currentPageId ?? '')
  return [Math.max(current, 0)]
}

export const usePrintStore = create<PrintStore>()((set, get) => ({
  dialogOpen: false,
  settings: { choice: 'all', range: '', fit: 'fit', resolution: 300 },
  progress: null,
  sheet: null,
  notice: null,

  openDialog() {
    if (useDocumentStore.getState().document.phase === 'ready') set({ dialogOpen: true, notice: null })
  },

  closeDialog() {
    if (!get().progress) set({ dialogOpen: false })
  },

  change(settings) {
    set({ settings: { ...get().settings, ...settings }, notice: null })
  },

  async print() {
    const { document } = useDocumentStore.getState()
    if (document.phase !== 'ready' || get().progress) return
    const indices = chosenPages(get().settings, document.pages.map((page) => page.id))
    if (!indices) {
      set({ notice: `Enter pages between 1 and ${document.pages.length}, like 1-3, 5` })
      return
    }
    await useEditingStore.getState().commit()
    const scale = get().settings.resolution / POINTS_PER_INCH
    const sheet: PrintedPage[] = []
    set({ progress: { done: 0, total: indices.length }, notice: null })
    try {
      for (const index of indices) {
        const page = document.pages[index]
        const [width, height] = [Math.round(page.width * scale), Math.round(page.height * scale)]
        const tile = await engine.request({ kind: 'renderTile', pageId: page.id, scale, x: 0, y: 0, width, height })
        sheet.push({ url: URL.createObjectURL(await CanvasPictureEncoder.encode(tile)), width: page.width, height: page.height })
        set({ progress: { done: sheet.length, total: indices.length } })
      }
      set({ dialogOpen: false, progress: null, sheet })
    } catch (error) {
      sheet.forEach((page) => URL.revokeObjectURL(page.url))
      set({ progress: null, notice: error instanceof CancelledError ? null : (error as Error).message })
    }
  },

  finish() {
    get().sheet?.forEach((page) => URL.revokeObjectURL(page.url))
    set({ sheet: null })
  },
}))
