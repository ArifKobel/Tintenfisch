import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { ProtectionDialogState } from '@/stores/ProtectionStore.ts'

export function useProtectionState(): ProtectionDialogState {
  const { protection } = useApplication()
  return useSyncExternalStore(protection.subscribe, protection.getSnapshot)
}
