import type { EngineClient } from '@engine/EngineClient.ts'
import type { Opened, OpenResult, PageSummary } from '@engine/protocol.ts'

export type ReadyDocument = { phase: 'ready'; name: string; session: number; pages: PageSummary[] } & Opened

export type DocumentState =
  | { phase: 'empty' }
  | { phase: 'loading'; name: string }
  | { phase: 'locked'; name: string; wrongPassword: boolean }
  | ReadyDocument
  | { phase: 'failed'; name: string; message: string }

type Listener = () => void

export class DocumentStore {
  readonly #client: EngineClient
  readonly #listeners = new Set<Listener>()
  readonly #replacedListeners = new Set<Listener>()
  #state: DocumentState = { phase: 'empty' }
  #session = 0

  constructor(client: EngineClient) {
    this.#client = client
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): DocumentState => this.#state

  onDocumentReplaced(listener: Listener): () => void {
    this.#replacedListeners.add(listener)
    return () => this.#replacedListeners.delete(listener)
  }

  async open(name: string, bytes: Uint8Array): Promise<void> {
    this.#update({ phase: 'loading', name })
    await this.#settle(name, () => this.#client.request({ kind: 'open', bytes }))
  }

  async unlock(password: string): Promise<void> {
    if (this.#state.phase !== 'locked') return
    const { name } = this.#state
    this.#update({ phase: 'loading', name })
    await this.#settle(name, () => this.#client.request({ kind: 'unlock', password }))
  }

  close() {
    if (this.#state.phase !== 'empty') this.#update({ phase: 'empty' })
  }

  replacePages(pages: PageSummary[]) {
    if (this.#state.phase === 'ready') this.#update({ ...this.#state, pages })
  }

  async refreshPages(): Promise<void> {
    if (this.#state.phase !== 'ready') return
    this.replacePages(await this.#client.request({ kind: 'pages' }))
  }

  async save(optimize = false): Promise<Uint8Array> {
    if (this.#state.phase !== 'ready') throw new Error('no document is open')
    return this.#client.request({ kind: 'save', optimize })
  }

  async #settle(name: string, opening: () => Promise<OpenResult>) {
    try {
      const result = await opening()
      if (result.status === 'locked') {
        this.#update({ phase: 'locked', name, wrongPassword: result.wrongPassword })
        return
      }
      const { status: _status, ...opened } = result
      const pages = await this.#client.request({ kind: 'pages' })
      this.#update({ phase: 'ready', name, session: ++this.#session, pages, ...opened })
    } catch (error) {
      this.#update({ phase: 'failed', name, message: (error as Error).message })
    }
  }

  #update(state: DocumentState) {
    const previous = this.#state
    this.#state = state
    this.#listeners.forEach((listener) => listener())
    const sameDocument = previous.phase === 'ready' && state.phase === 'ready' && previous.session === state.session
    if (!sameDocument) this.#replacedListeners.forEach((listener) => listener())
  }
}
