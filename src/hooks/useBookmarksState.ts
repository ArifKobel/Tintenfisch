import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { BookmarksState } from '@/stores/BookmarksStore.ts'

export function useBookmarksState(): BookmarksState {
  const { bookmarks } = useApplication()
  return useSyncExternalStore(bookmarks.subscribe, bookmarks.getSnapshot)
}
