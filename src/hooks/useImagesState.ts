import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { ImagesState } from '@/stores/ImagesStore.ts'

export function useImagesState(): ImagesState {
  const { images } = useApplication()
  return useSyncExternalStore(images.subscribe, images.getSnapshot)
}
