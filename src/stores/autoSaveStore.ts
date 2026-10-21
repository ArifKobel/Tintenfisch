import { create } from 'zustand'
import { documentLibrary } from '@/services/DocumentLibrary.ts'
import { engine } from '@/services/EngineClient.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'

export const SAVE_DELAY = 1000

export type SaveStatus = 'idle' | 'pending' | 'saving' | 'saved' | 'failed'

interface AutoSaveStore {
  status: SaveStatus
  error: string | null
  track(file: string | null): void
  flush(): Promise<void>
}

export const useAutoSaveStore = create<AutoSaveStore>()((set, get) => {
  let file: string | null = null
  let savedRevision = ''
  let timer: ReturnType<typeof setTimeout> | null = null
  let queue: Promise<void> = Promise.resolve()

  const cancel = () => {
    if (timer !== null) clearTimeout(timer)
    timer = null
  }

  const save = async () => {
    const saving = file
    const revision = engine.revision
    if (!saving || revision === savedRevision) return
    set({ status: 'saving', error: null })
    try {
      await documentLibrary.replace(saving, await useDocumentStore.getState().save())
      if (saving !== file) return
      savedRevision = revision
      set({ status: timer !== null ? 'pending' : 'saved', error: null })
    } catch (error) {
      if (saving === file) set({ status: 'failed', error: (error as Error).message })
    }
  }

  const enqueue = () => {
    queue = queue.then(save)
  }

  engine.onChange(() => {
    if (!file || engine.revision === savedRevision) return
    cancel()
    timer = setTimeout(() => {
      timer = null
      enqueue()
    }, SAVE_DELAY)
    if (get().status !== 'saving') set({ status: 'pending', error: null })
  })

  return {
    status: 'idle',
    error: null,

    track(tracked) {
      cancel()
      file = tracked
      savedRevision = engine.revision
      set({ status: tracked ? 'saved' : 'idle', error: null })
    },

    flush() {
      if (timer !== null) {
        cancel()
        enqueue()
      }
      return queue
    },
  }
})
