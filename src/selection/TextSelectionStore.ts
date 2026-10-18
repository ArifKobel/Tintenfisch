import { CancelledError, type EngineClient } from '@engine/EngineClient.ts'
import type { SelectionUnit, TextPoint, TextSelection } from '@engine/protocol.ts'

export type SelectionState = ({ pageId: string; unit: SelectionUnit } & TextSelection) | null

type Listener = () => void

export class TextSelectionStore {
  readonly #client: EngineClient
  readonly #listeners = new Set<Listener>()
  #state: SelectionState = null
  #anchor: { pageId: string; point: TextPoint; unit: SelectionUnit } | null = null
  #sequence = 0

  constructor(client: EngineClient) {
    this.#client = client
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): SelectionState => this.#state

  get text(): string {
    return this.#state?.text ?? ''
  }

  async begin(pageId: string, x: number, y: number, unit: SelectionUnit) {
    await TextSelectionStore.#unlessCancelled(() => this.#begin(pageId, x, y, unit))
  }

  async extend(pageId: string, x: number, y: number) {
    await TextSelectionStore.#unlessCancelled(() => this.#extend(pageId, x, y))
  }

  static async #unlessCancelled(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) throw error
    }
  }

  async #begin(pageId: string, x: number, y: number, unit: SelectionUnit) {
    const sequence = ++this.#sequence
    const point = await this.#client.request({ kind: 'hitText', pageId, x, y })
    if (sequence !== this.#sequence) return
    if (!point) {
      this.clear()
      return
    }
    this.#anchor = { pageId, point, unit }
    await this.#select(pageId, point, point, unit, sequence)
  }

  async #extend(pageId: string, x: number, y: number) {
    const anchor = this.#anchor
    if (!anchor || anchor.pageId !== pageId) return
    const sequence = ++this.#sequence
    const point = await this.#client.request({ kind: 'hitText', pageId, x, y }, { group: 'selection-hit' })
    if (sequence !== this.#sequence || !point) return
    await this.#select(pageId, anchor.point, point, anchor.unit, sequence)
  }

  clear() {
    this.#sequence++
    this.#anchor = null
    this.#update(null)
  }

  async #select(pageId: string, anchor: TextPoint, focus: TextPoint, unit: SelectionUnit, sequence: number) {
    const selection = await this.#client.request({ kind: 'selectText', pageId, anchor, focus, unit }, { group: 'selection' })
    if (sequence === this.#sequence) this.#update({ pageId, unit, ...selection })
  }

  #update(state: SelectionState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
