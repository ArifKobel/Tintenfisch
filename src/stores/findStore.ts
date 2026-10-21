import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentChanged } from '@/stores/documentStore.ts'
import type { FindMatch } from '@/types/protocol.ts'

interface FindStore {
  query: string
  matches: FindMatch[]
  current: number
  currentMatch(): FindMatch | undefined
  search(query: string): Promise<void>
  refresh(): Promise<void>
  next(): void
  previous(): void
}

export const useFindStore = create<FindStore>()((set, get) => {
  let sequence = 0

  const run = async (query: string, current: number) => {
    const searching = ++sequence
    if (query.trim() === '') {
      set({ query, matches: [], current: 0 })
      return
    }
    set({ query })
    try {
      const matches = await engine.request({ kind: 'find', query }, { group: 'find' })
      if (searching === sequence) set({ query, matches, current: Math.min(current, Math.max(matches.length - 1, 0)) })
    } catch (error) {
      if (!(error instanceof CancelledError)) throw error
    }
  }

  const step = (direction: number) => {
    const { matches, current } = get()
    if (matches.length > 0) set({ current: (current + direction + matches.length) % matches.length })
  }

  return {
    query: '',
    matches: [],
    current: 0,

    currentMatch() {
      const { matches, current } = get()
      return matches[current]
    },

    search: (query) => run(query, 0),

    async refresh() {
      const { query, current } = get()
      if (query.trim() !== '') await run(query, current)
    },

    next: () => step(1),

    previous: () => step(-1),
  }
})

onDocumentChanged(() => void useFindStore.getState().refresh())
