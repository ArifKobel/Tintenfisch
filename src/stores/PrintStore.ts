import type { DocumentStore } from '@/stores/DocumentStore.ts'
import type { EditingStore } from '@/stores/EditingStore.ts'
import { CancelledError, type EngineClient } from '@/services/EngineClient.ts'
import type { RenderedTile } from '@/types/protocol.ts'
import { PageRange } from '@/lib/PageRange.ts'
import type { ViewportStore } from '@/stores/ViewportStore.ts'

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

export interface PrintState {
  dialogOpen: boolean
  settings: PrintSettings
  progress: { done: number; total: number } | null
  sheet: readonly PrintedPage[] | null
  notice: string | null
}

export type PictureEncoder = (tile: RenderedTile) => Promise<Blob>

const POINTS_PER_INCH = 72

type Listener = () => void

export class PrintStore {
  readonly #client: EngineClient
  readonly #documents: DocumentStore
  readonly #viewport: ViewportStore
  readonly #editing: EditingStore
  readonly #encode: PictureEncoder
  readonly #listeners = new Set<Listener>()
  #state: PrintState = {
    dialogOpen: false,
    settings: { choice: 'all', range: '', fit: 'fit', resolution: 300 },
    progress: null,
    sheet: null,
    notice: null,
  }

  constructor(client: EngineClient, documents: DocumentStore, viewport: ViewportStore, editing: EditingStore, encode: PictureEncoder) {
    this.#client = client
    this.#documents = documents
    this.#viewport = viewport
    this.#editing = editing
    this.#encode = encode
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): PrintState => this.#state

  openDialog() {
    if (this.#documents.getSnapshot().phase === 'ready') this.#update({ ...this.#state, dialogOpen: true, notice: null })
  }

  closeDialog() {
    if (!this.#state.progress) this.#update({ ...this.#state, dialogOpen: false })
  }

  change(settings: Partial<PrintSettings>) {
    this.#update({ ...this.#state, settings: { ...this.#state.settings, ...settings }, notice: null })
  }

  async print() {
    const document = this.#documents.getSnapshot()
    if (document.phase !== 'ready' || this.#state.progress) return
    const indices = this.#chosenPages(document.pages.map((page) => page.id))
    if (!indices) {
      this.#update({ ...this.#state, notice: `Enter pages between 1 and ${document.pages.length}, like 1-3, 5` })
      return
    }
    await this.#editing.commit()
    const scale = this.#state.settings.resolution / POINTS_PER_INCH
    const sheet: PrintedPage[] = []
    this.#update({ ...this.#state, progress: { done: 0, total: indices.length }, notice: null })
    try {
      for (const index of indices) {
        const page = document.pages[index]
        const [width, height] = [Math.round(page.width * scale), Math.round(page.height * scale)]
        const tile = await this.#client.request({ kind: 'renderTile', pageId: page.id, scale, x: 0, y: 0, width, height })
        sheet.push({ url: URL.createObjectURL(await this.#encode(tile)), width: page.width, height: page.height })
        this.#update({ ...this.#state, progress: { done: sheet.length, total: indices.length } })
      }
      this.#update({ ...this.#state, dialogOpen: false, progress: null, sheet })
    } catch (error) {
      sheet.forEach((page) => URL.revokeObjectURL(page.url))
      const notice = error instanceof CancelledError ? null : (error as Error).message
      this.#update({ ...this.#state, progress: null, notice })
    }
  }

  finish() {
    this.#state.sheet?.forEach((page) => URL.revokeObjectURL(page.url))
    this.#update({ ...this.#state, sheet: null })
  }

  #chosenPages(pageIds: string[]): number[] | null {
    const { choice, range } = this.#state.settings
    if (choice === 'all') return pageIds.map((_, index) => index)
    if (choice === 'range') return PageRange.parse(range, pageIds.length)
    const current = pageIds.indexOf(this.#viewport.getSnapshot().currentPageId ?? '')
    return [Math.max(current, 0)]
  }

  #update(state: PrintState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
