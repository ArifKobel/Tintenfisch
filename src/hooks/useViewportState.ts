import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { ViewportState } from '@/stores/ViewportStore.ts'

export function useViewportState(): ViewportState {
  const { viewport } = useApplication()
  return useSyncExternalStore(viewport.subscribe, viewport.getSnapshot)
}
