import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { OrganizeState } from '@/stores/OrganizeStore.ts'

export function useOrganizeState(): OrganizeState {
  const { organize } = useApplication()
  return useSyncExternalStore(organize.subscribe, organize.getSnapshot)
}
