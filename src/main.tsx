import { createRouter, RouterProvider } from '@tanstack/react-router'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { routeTree } from '@/routeTree.gen.ts'
import { engine } from '@/services/EngineClient.ts'
import { useAutoSaveStore } from '@/stores/autoSaveStore.ts'
import EngineWorker from '@/worker/engine.worker.ts?worker'
import '@/styles/tokens.css'
import '@/styles/controls.css'
import '@/styles/panels.css'
import '@/styles/dialogs.css'
import '@/styles/shell.css'
import '@/styles/tools.css'
import '@/styles/viewer.css'
import '@/styles/document.css'
import '@/styles/files.css'
import '@/styles/selection.css'
import '@/styles/find.css'
import '@/styles/editing.css'
import '@/styles/comments.css'
import '@/styles/forms.css'
import '@/styles/navigation.css'
import '@/styles/images.css'
import '@/styles/signing.css'
import '@/styles/protection.css'
import '@/styles/printing.css'
import '@/styles/thumbnails.css'

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
