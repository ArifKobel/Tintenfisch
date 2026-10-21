import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentReplaced } from '@/stores/documentStore.ts'
import type { Permissions, ProtectionState } from '@/types/protocol.ts'

const ENTROPY = 68

interface ProtectionStore {
  dialogOpen: boolean
  current: ProtectionState | null
  notice: string | null
  pending: string | null
  openDialog(): Promise<void>
  closeDialog(): void
  grantOwnerRights(password: string): Promise<void>
  protect(userPassword: string, ownerPassword: string, permissions: Permissions): Promise<void>
  unprotect(): Promise<void>
  reset(): void
}

export const useProtectionStore = create<ProtectionStore>()((set) => {
  const attempt = async (work: () => Promise<void>) => {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) set({ notice: (error as Error).message })
    }
  }

  const finish = (pending: string) => set({ dialogOpen: false, notice: null, pending })

  return {
    dialogOpen: false,
    current: null,
    notice: null,
    pending: null,

    async openDialog() {
      await attempt(async () => set({ dialogOpen: true, current: await engine.request({ kind: 'protection' }), notice: null }))
    },

    closeDialog() {
      set({ dialogOpen: false, notice: null })
    },

    async grantOwnerRights(password) {
      await attempt(async () => {
        const granted = await engine.request({ kind: 'grantOwnerRights', password })
        const current = await engine.request({ kind: 'protection' })
        set({ current, notice: granted ? null : 'That is not the permissions password' })
      })
    },

    async protect(userPassword, ownerPassword, permissions) {
      await attempt(async () => {
        const entropy = crypto.getRandomValues(new Uint8Array(ENTROPY))
        await engine.request({ kind: 'protect', userPassword, ownerPassword, permissions, entropy })
        finish('The protection is written when you save the document')
      })
    },

    async unprotect() {
      await attempt(async () => {
        await engine.request({ kind: 'unprotect' })
        finish('The document is saved without protection from now on')
      })
    },

    reset() {
      set({ dialogOpen: false, current: null, notice: null, pending: null })
    },
  }
})

onDocumentReplaced(() => useProtectionStore.getState().reset())
