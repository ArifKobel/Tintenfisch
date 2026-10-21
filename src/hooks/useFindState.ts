import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { FindState } from '@/stores/FindStore.ts'

export function useFindState(): FindState {
  const { find } = useApplication()
  return useSyncExternalStore(find.subscribe, find.getSnapshot)
}
