import { createFileRoute } from '@tanstack/react-router'
import { HomeScreen } from '@/components/home/HomeScreen.tsx'
import { useLibraryStore } from '@/stores/libraryStore.ts'

export const Route = createFileRoute('/')({
  loader: () => useLibraryStore.getState().goHome(),
  component: Home,
})

function Home() {
  return (
    <section className="flex min-h-0 min-w-0 flex-1 items-center justify-center">
      <HomeScreen />
    </section>
  )
}
