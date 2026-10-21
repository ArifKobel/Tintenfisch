import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { PanelState } from '@/stores/PanelStore.ts'

export function usePanelState(): PanelState {
  const { panels } = useApplication()
  return useSyncExternalStore(panels.subscribe, panels.getSnapshot)
}
