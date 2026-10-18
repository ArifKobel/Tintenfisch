import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from '@app/App.tsx'
import { Application } from '@app/Application.ts'
import { ApplicationContext } from '@app/ApplicationContext.ts'
import EngineWorker from '@engine/engine.worker.ts?worker'
import '@styles/app.css'

const application = new Application(new EngineWorker())

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ApplicationContext value={application}>
      <App />
    </ApplicationContext>
  </StrictMode>,
)
