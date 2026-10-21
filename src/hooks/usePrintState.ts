import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { PrintState } from '@/stores/PrintStore.ts'

export function usePrintState(): PrintState {
  const { printing } = useApplication()
  return useSyncExternalStore(printing.subscribe, printing.getSnapshot)
}
