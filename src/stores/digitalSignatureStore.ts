import { create } from 'zustand'
import { pdfDate } from '@/lib/pdfDate.ts'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'
import type { SignatureInfo, SigningRequest } from '@/types/protocol.ts'

interface DigitalSignatureStore {
  dialogOpen: boolean
  signing: boolean
  notice: string | null
  reports: SignatureInfo[]
  reload(): Promise<void>
  openDialog(): void
  closeDialog(): void
  sign(request: SigningRequest): Promise<{ name: string; bytes: Uint8Array } | null>
  reset(): void
}

export function signedName(name: string): string {
  const base = name.replace(/\.pdf$/i, '')
  return `${base.replace(/-signed$/, '')}-signed.pdf`
}

export const useDigitalSignatureStore = create<DigitalSignatureStore>()((set, get) => ({
  dialogOpen: false,
  signing: false,
  notice: null,
  reports: [],

  async reload() {
    try {
      set({ reports: await engine.request({ kind: 'signatures' }, { group: 'signatures' }) })
    } catch (error) {
      if (!(error instanceof CancelledError)) set({ notice: (error as Error).message })
    }
  },

  openDialog() {
    set({ dialogOpen: true, notice: null })
  },

  closeDialog() {
    if (!get().signing) set({ dialogOpen: false })
  },

  async sign(request) {
    const { document } = useDocumentStore.getState()
    if (document.phase !== 'ready' || get().signing) return null
    set({ signing: true, notice: null })
    try {
      const bytes = await engine.request({ kind: 'signField', ...request, date: pdfDate(new Date()) })
      const name = signedName(document.name)
      set({ signing: false, dialogOpen: false })
      await useDocumentStore.getState().open(name, bytes.slice())
      return { name, bytes }
    } catch (error) {
      set({ signing: false, notice: error instanceof CancelledError ? null : (error as Error).message })
      return null
    }
  },

  reset() {
    set({ dialogOpen: false, signing: false, notice: null, reports: [] })
  },
}))

onDocumentReplaced((ready) => {
  useDigitalSignatureStore.getState().reset()
  if (ready) void useDigitalSignatureStore.getState().reload()
})
