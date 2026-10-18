import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { LinksState } from '@navigation/LinksStore.ts'

export function useLinksState(): LinksState {
  const { links } = useApplication()
  return useSyncExternalStore(links.subscribe, links.getSnapshot)
}
