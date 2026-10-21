import { useNavigate } from '@tanstack/react-router'
import { useCallback } from 'react'
import { useLibraryStore } from '@/stores/libraryStore.ts'

export function useOpenFile(): (name: string, bytes: Uint8Array) => Promise<void> {
  const navigate = useNavigate()
  return useCallback(
    async (name: string, bytes: Uint8Array) => {
      const id = await useLibraryStore.getState().openNew(name, bytes)
      if (id) await navigate({ to: '/documents/$documentId', params: { documentId: id } })
    },
    [navigate],
  )
}
