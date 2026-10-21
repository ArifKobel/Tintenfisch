import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { DocumentState } from '@/stores/DocumentStore.ts'

export function useDocumentState(): DocumentState {
  const { documents } = useApplication()
  return useSyncExternalStore(documents.subscribe, documents.getSnapshot)
}
