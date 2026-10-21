import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { EditingState } from '@/stores/EditingStore.ts'

export function useEditingState(): EditingState {
  const { editing } = useApplication()
  return useSyncExternalStore(editing.subscribe, editing.getSnapshot)
}
