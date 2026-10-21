import { CancelledError, type EngineClient } from '@/services/EngineClient.ts'
import type { FindMatch } from '@/types/protocol.ts'

export interface FindState {
  query: string
  matches: FindMatch[]
  current: number
}

type Listener = () => void

export class FindStore {
  readonly #client: EngineClient
  readonly #listeners = new Set<Listener>()
  #state: FindState = { query: '', matches: [], current: 0 }
  #sequence = 0

  constructor(client: EngineClient) {
    this.#client = client
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): FindState => this.#state

  async search(query: string) {
    await this.#run(query, 0)
  }

  async refresh() {
    if (this.#state.query.trim() !== '') await this.#run(this.#state.query, this.#state.current)
  }

  async #run(query: string, current: number) {
    const sequence = ++this.#sequence
    if (query.trim() === '') {
      this.#update({ query, matches: [], current: 0 })
      return
    }
    this.#update({ ...this.#state, query })
    try {
      const matches = await this.#client.request({ kind: 'find', query }, { group: 'find' })
      if (sequence === this.#sequence) this.#update({ query, matches, current: Math.min(current, Math.max(matches.length - 1, 0)) })
    } catch (error) {
      if (!(error instanceof CancelledError)) throw error
    }
  }

  next() {
    this.#step(1)
  }

  previous() {
    this.#step(-1)
  }

  get currentMatch(): FindMatch | undefined {
    return this.#state.matches[this.#state.current]
  }

  #step(direction: number) {
    const count = this.#state.matches.length
    if (count === 0) return
    this.#update({ ...this.#state, current: (this.#state.current + direction + count) % count })
  }

  #update(state: FindState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
