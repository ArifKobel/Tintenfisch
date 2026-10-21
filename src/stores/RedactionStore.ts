import { CancelledError, type EngineClient } from '@/services/EngineClient.ts'
import type { PageRevisionStore } from '@/stores/PageRevisionStore.ts'

export interface RedactionState {
  confirming: boolean
  applying: boolean
  notice: string | null
}

type Listener = () => void

const INITIAL: RedactionState = { confirming: false, applying: false, notice: null }

export class RedactionStore {
  readonly #client: EngineClient
  readonly #revisions: PageRevisionStore
  readonly #listeners = new Set<Listener>()
  readonly #changeListeners = new Set<Listener>()
  #state: RedactionState = INITIAL

  constructor(client: EngineClient, revisions: PageRevisionStore) {
    this.#client = client
    this.#revisions = revisions
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): RedactionState => this.#state

  onDocumentChange(listener: Listener): () => void {
    this.#changeListeners.add(listener)
    return () => this.#changeListeners.delete(listener)
  }

  requestApply() {
    if (!this.#state.applying) this.#update({ ...this.#state, confirming: true, notice: null })
  }

  cancel() {
    this.#update({ ...this.#state, confirming: false })
  }

  async apply() {
    if (!this.#state.confirming || this.#state.applying) return
    this.#update({ ...this.#state, confirming: false, applying: true })
    try {
      const change = await this.#client.request({ kind: 'applyRedactions' })
      this.#revisions.touch(change.dirtyPages)
      this.#update({ ...this.#state, applying: false })
      this.#changeListeners.forEach((listener) => listener())
    } catch (error) {
      const notice = error instanceof CancelledError ? null : (error as Error).message
      this.#update({ ...this.#state, applying: false, notice })
    }
  }

  reset() {
    this.#update(INITIAL)
  }

  #update(state: RedactionState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
