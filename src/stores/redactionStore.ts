import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'

interface RedactionStore {
  confirming: boolean
  applying: boolean
  notice: string | null
  requestApply(): void
  cancel(): void
  apply(): Promise<void>
  reset(): void
}

export const useRedactionStore = create<RedactionStore>()((set, get) => ({
  confirming: false,
  applying: false,
  notice: null,

  requestApply() {
    if (!get().applying) set({ confirming: true, notice: null })
  },

  cancel() {
    set({ confirming: false })
  },

  async apply() {
    if (!get().confirming || get().applying) return
    set({ confirming: false, applying: true })
    try {
      const change = await engine.request({ kind: 'applyRedactions' })
      const documents = useDocumentStore.getState()
      documents.touchPages(change.dirtyPages)
      set({ applying: false })
      documents.markChanged()
    } catch (error) {
      set({ applying: false, notice: error instanceof CancelledError ? null : (error as Error).message })
    }
  },

  reset() {
    set({ confirming: false, applying: false, notice: null })
  },
}))

onDocumentReplaced(() => useRedactionStore.getState().reset())
