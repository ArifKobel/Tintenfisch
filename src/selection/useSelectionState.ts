import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { SelectionState } from '@selection/TextSelectionStore.ts'

export function useSelectionState(): SelectionState {
  const { selection } = useApplication()
  return useSyncExternalStore(selection.subscribe, selection.getSnapshot)
}
