import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { AutoSaveState } from '@/stores/AutoSave.ts'

export function useAutoSaveState(): AutoSaveState {
  const { autoSave } = useApplication()
  return useSyncExternalStore(autoSave.subscribe, autoSave.getSnapshot)
}
