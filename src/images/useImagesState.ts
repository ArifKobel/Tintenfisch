import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { ImagesState } from '@images/ImagesStore.ts'

export function useImagesState(): ImagesState {
  const { images } = useApplication()
  return useSyncExternalStore(images.subscribe, images.getSnapshot)
}
