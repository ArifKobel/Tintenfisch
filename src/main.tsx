import { createRouter, RouterProvider } from '@tanstack/react-router'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { routeTree } from '@/routeTree.gen.ts'
import { engine } from '@/services/EngineClient.ts'
import { useAutoSaveStore } from '@/stores/autoSaveStore.ts'
import EngineWorker from '@/worker/engine.worker.ts?worker'
import '@/styles/app.css'

const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

engine.connect(new EngineWorker())
window.addEventListener('pagehide', () => void useAutoSaveStore.getState().flush())

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
)
