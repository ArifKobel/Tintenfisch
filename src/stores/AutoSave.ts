import type { DocumentStore } from '@/stores/DocumentStore.ts'
import type { EngineClient } from '@/services/EngineClient.ts'
import type { DocumentLibrary } from '@/services/DocumentLibrary.ts'

export const SAVE_DELAY = 1000

export interface AutoSaveState {
  status: 'idle' | 'pending' | 'saving' | 'saved' | 'failed'
  error: string | null
}

export type Schedule = (run: () => void, delay: number) => () => void

type Listener = () => void

const later: Schedule = (run, delay) => {
  const timer = setTimeout(run, delay)
  return () => clearTimeout(timer)
}

export class AutoSave {
  readonly #client: EngineClient
  readonly #documents: DocumentStore
  readonly #library: DocumentLibrary
  readonly #schedule: Schedule
  readonly #listeners = new Set<Listener>()
  #state: AutoSaveState = { status: 'idle', error: null }
  #file: string | null = null
  #savedRevision = ''
  #cancelTimer: (() => void) | null = null
  #queue: Promise<void> = Promise.resolve()

  constructor(client: EngineClient, documents: DocumentStore, library: DocumentLibrary, schedule: Schedule = later) {
    this.#client = client
    this.#documents = documents
    this.#library = library
    this.#schedule = schedule
    client.onChange(() => this.#changed())
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): AutoSaveState => this.#state

  track(file: string | null) {
    this.#cancel()
    this.#file = file
    this.#savedRevision = this.#client.revision
    this.#update({ status: file ? 'saved' : 'idle', error: null })
  }

  flush(): Promise<void> {
    if (this.#cancelTimer) {
      this.#cancel()
      this.#enqueue()
    }
    return this.#queue
  }

  #changed() {
    if (!this.#file || this.#client.revision === this.#savedRevision) return
    this.#cancel()
    this.#cancelTimer = this.#schedule(() => {
      this.#cancelTimer = null
      this.#enqueue()
    }, SAVE_DELAY)
    if (this.#state.status !== 'saving') this.#update({ status: 'pending', error: null })
  }

  #enqueue() {
    this.#queue = this.#queue.then(() => this.#save())
  }

  async #save() {
    const file = this.#file
    const revision = this.#client.revision
    if (!file || revision === this.#savedRevision) return
    this.#update({ status: 'saving', error: null })
    try {
      await this.#library.replace(file, await this.#documents.save())
      if (file !== this.#file) return
      this.#savedRevision = revision
      this.#update({ status: this.#cancelTimer ? 'pending' : 'saved', error: null })
    } catch (error) {
      if (file === this.#file) this.#update({ status: 'failed', error: (error as Error).message })
    }
  }

  #cancel() {
    this.#cancelTimer?.()
    this.#cancelTimer = null
  }

  #update(state: AutoSaveState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
