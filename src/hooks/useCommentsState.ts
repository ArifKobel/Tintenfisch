import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { CommentsState } from '@/stores/CommentsStore.ts'

export function useCommentsState(): CommentsState {
  const { comments } = useApplication()
  return useSyncExternalStore(comments.subscribe, comments.getSnapshot)
}
