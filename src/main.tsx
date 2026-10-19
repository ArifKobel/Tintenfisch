import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from '@app/App.tsx'
import { Application } from '@app/Application.ts'
import { ApplicationContext } from '@app/ApplicationContext.ts'
import EngineWorker from '@engine/engine.worker.ts?worker'
import '@styles/tokens.css'
import '@ui/controls.css'
import '@ui/panels.css'
import '@ui/dialogs.css'
import '@shell/shell.css'
import '@tools/tools.css'
import '@viewer/viewer.css'
import '@document/document.css'
import '@files/files.css'
import '@selection/selection.css'
import '@find/find.css'
import '@editing/editing.css'
import '@comments/comments.css'
import '@forms/forms.css'
import '@navigation/navigation.css'
import '@images/images.css'
import '@signing/signing.css'
import '@protection/protection.css'
import '@printing/printing.css'
import '@thumbnails/thumbnails.css'

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
