import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from '@/app/App.tsx'
import { Application } from '@/app/Application.ts'
import { ApplicationContext } from '@/app/ApplicationContext.ts'
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

function localStore(): Storage | null {
  try {
    return window.localStorage
  } catch {
    return null
  }
}

const application = new Application(new EngineWorker(), { indexedDB, window, storage: localStore(), fetch: window.fetch.bind(window) })
void application.files.start()
window.addEventListener('pagehide', () => void application.autoSave.flush())

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ApplicationContext value={application}>
      <App />
    </ApplicationContext>
  </StrictMode>,
)
