import { useCallback, useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'

export function usePageRevision(pageId: string): number {
  const { revisions } = useApplication()
  const revision = useCallback(() => revisions.revisionOf(pageId), [revisions, pageId])
  return useSyncExternalStore(revisions.subscribe, revision)
}
