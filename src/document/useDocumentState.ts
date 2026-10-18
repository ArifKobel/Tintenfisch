import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { DocumentState } from '@document/DocumentStore.ts'

export function useDocumentState(): DocumentState {
  const { documents } = useApplication()
  return useSyncExternalStore(documents.subscribe, documents.getSnapshot)
}
