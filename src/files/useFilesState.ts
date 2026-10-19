import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { FilesState } from '@files/FilesStore.ts'

export function useFilesState(): FilesState {
  const { files } = useApplication()
  return useSyncExternalStore(files.subscribe, files.getSnapshot)
}
