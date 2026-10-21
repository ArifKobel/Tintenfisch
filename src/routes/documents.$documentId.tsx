import { createFileRoute, redirect } from '@tanstack/react-router'
import { DocumentView } from '@/components/viewer/DocumentView.tsx'
import { useLibraryStore } from '@/stores/libraryStore.ts'

export const Route = createFileRoute('/documents/$documentId')({
  loader: async ({ params }) => {
    if (!(await useLibraryStore.getState().show(params.documentId))) throw redirect({ to: '/', replace: true })
  },
  component: DocumentView,
})
