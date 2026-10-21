import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { onToolChosen } from '@/stores/workspaceStore.ts'
import type { Box, SignatureMarkRequest } from '@/types/protocol.ts'

interface SignatureStore {
  padOpen: boolean
  mark: SignatureMarkRequest | null
  notice: string | null
  openPad(): void
  closePad(): void
  adopt(mark: SignatureMarkRequest): void
  refuse(notice: string): void
  place(pageId: string, rect: Box): Promise<void>
}

export const useSignatureStore = create<SignatureStore>()((set, get) => ({
  padOpen: false,
  mark: null,
  notice: null,

  openPad() {
    set({ padOpen: true, notice: null })
  },

  closePad() {
    set({ padOpen: false })
  },

  adopt(mark) {
    set({ mark, padOpen: false, notice: null })
  },

  refuse(notice) {
    set({ notice })
  },

  async place(pageId, rect) {
    const { mark } = get()
    if (!mark) return
    try {
      const added = await engine.request({ kind: 'addSignature', pageId, rect, name: 'Signature', mark })
      const documents = useDocumentStore.getState()
      documents.touchPages(added.dirtyPages)
      set({ notice: null })
      documents.markChanged()
    } catch (error) {
      if (!(error instanceof CancelledError)) get().refuse((error as Error).message)
    }
  },
}))

onToolChosen((tool) => {
  if (tool === 'sign' && !useSignatureStore.getState().mark) useSignatureStore.getState().openPad()
})
