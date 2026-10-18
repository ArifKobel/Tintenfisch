import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { RedactionState } from '@redaction/RedactionStore.ts'

export function useRedactionState(): RedactionState {
  const { redaction } = useApplication()
  return useSyncExternalStore(redaction.subscribe, redaction.getSnapshot)
}
