import { CancelledError, type EngineClient } from '@engine/EngineClient.ts'
import type { DocumentChange, Highlight } from '@engine/protocol.ts'
import type { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

export interface ImageChoice {
  pageId: string
  index: number
}

export interface ImagesState {
  pages: Readonly<Record<string, readonly Highlight[]>>
  selected: ImageChoice | null
  notice: string | null
}

type Listener = () => void

const BACKGROUND_SHARE = 0.5

export class ImagesStore {
  readonly #client: EngineClient
  readonly #revisions: PageRevisionStore
  readonly #listeners = new Set<Listener>()
  #state: ImagesState = { pages: {}, selected: null, notice: null }

  constructor(client: EngineClient, revisions: PageRevisionStore) {
    this.#client = client
    this.#revisions = revisions
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): ImagesState => this.#state

  get selectedFrame(): Highlight | null {
    const { selected, pages } = this.#state
    return selected ? (pages[selected.pageId]?.[selected.index] ?? null) : null
  }

  async load(pageId: string) {
    await this.#attempt(async () => {
      const frames = await this.#client.request({ kind: 'images', pageId }, { group: `images-${pageId}` })
      const { selected } = this.#state
      const kept = selected?.pageId !== pageId || selected.index < frames.length ? selected : null
      this.#update({ ...this.#state, pages: { ...this.#state.pages, [pageId]: frames }, selected: kept })
    })
  }

  select(choice: ImageChoice | null) {
    const { selected } = this.#state
    if (choice?.pageId === selected?.pageId && choice?.index === selected?.index) return
    this.#update({ ...this.#state, selected: choice, notice: null })
  }

  selectAt(pageId: string, x: number, y: number) {
    const index = this.#indexAt(pageId, x, y, Infinity)
    this.select(index === null ? null : { pageId, index })
  }

  foregroundAt(pageId: string, x: number, y: number, pageArea: number): number | null {
    return this.#indexAt(pageId, x, y, pageArea * BACKGROUND_SHARE)
  }

  #indexAt(pageId: string, x: number, y: number, largestArea: number): number | null {
    const frames = this.#state.pages[pageId] ?? []
    const index = frames.findLastIndex((frame) => frame.width * frame.height < largestArea && ImagesStore.#contains(frame, x, y))
    return index < 0 ? null : index
  }

  async insert(pageId: string, bytes: Uint8Array, x: number, y: number) {
    await this.#attempt(async () => {
      const { index, dirtyPages } = await this.#client.request({ kind: 'insertImage', pageId, bytes, x, y })
      await this.#changed(pageId, { dirtyPages })
      this.select({ pageId, index })
    })
  }

  async move(choice: ImageChoice, frame: Highlight) {
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind: 'moveImage', ...choice, frame })
      await this.#changed(choice.pageId, change)
    })
  }

  async replaceSelected(bytes: Uint8Array) {
    const choice = this.#state.selected
    if (!choice) return
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind: 'replaceImage', ...choice, bytes })
      await this.#changed(choice.pageId, change)
    })
  }

  async removeSelected() {
    const choice = this.#state.selected
    if (!choice) return
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind: 'deleteImage', ...choice })
      this.#update({ ...this.#state, selected: null })
      await this.#changed(choice.pageId, change)
    })
  }

  reset() {
    this.#update({ pages: {}, selected: null, notice: null })
  }

  async #changed(pageId: string, change: DocumentChange) {
    this.#revisions.touch(change.dirtyPages)
    await this.load(pageId)
  }

  static #contains(frame: Highlight, x: number, y: number): boolean {
    const [dx, dy] = [x - frame.x, y - frame.y]
    const along = dx * Math.cos(frame.angle) + dy * Math.sin(frame.angle)
    const down = -dx * Math.sin(frame.angle) + dy * Math.cos(frame.angle)
    return along >= 0 && along <= frame.width && down >= 0 && down <= frame.height
  }

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: ImagesState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
