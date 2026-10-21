import { CancelledError, type EngineClient } from '@/services/EngineClient.ts'
import type { Box, SignatureMarkRequest } from '@/types/protocol.ts'
import type { PageRevisionStore } from '@/stores/PageRevisionStore.ts'

export interface SignatureState {
  padOpen: boolean
  mark: SignatureMarkRequest | null
  notice: string | null
}

type Listener = () => void

export class SignatureStore {
  readonly #client: EngineClient
  readonly #revisions: PageRevisionStore
  readonly #listeners = new Set<Listener>()
  readonly #changeListeners = new Set<Listener>()
  #state: SignatureState = { padOpen: false, mark: null, notice: null }

  constructor(client: EngineClient, revisions: PageRevisionStore) {
    this.#client = client
    this.#revisions = revisions
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): SignatureState => this.#state

  onDocumentChange(listener: Listener): () => void {
    this.#changeListeners.add(listener)
    return () => this.#changeListeners.delete(listener)
  }

  openPad() {
    this.#update({ ...this.#state, padOpen: true, notice: null })
  }

  closePad() {
    this.#update({ ...this.#state, padOpen: false })
  }

  adopt(mark: SignatureMarkRequest) {
    this.#update({ ...this.#state, mark, padOpen: false, notice: null })
  }

  refuse(notice: string) {
    this.#update({ ...this.#state, notice })
  }

  async place(pageId: string, rect: Box) {
    const mark = this.#state.mark
    if (!mark) return
    try {
      const added = await this.#client.request({ kind: 'addSignature', pageId, rect, name: 'Signature', mark })
      this.#revisions.touch(added.dirtyPages)
      this.#update({ ...this.#state, notice: null })
      this.#changeListeners.forEach((listener) => listener())
    } catch (error) {
      if (!(error instanceof CancelledError)) this.refuse((error as Error).message)
    }
  }

  #update(state: SignatureState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
