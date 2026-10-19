import { useApplication } from '@app/ApplicationContext.ts'
import { DocumentTitle } from '@document/DocumentTitle.tsx'
import { OpenDocumentButton } from '@document/OpenDocumentButton.tsx'
import { SaveDocumentButton } from '@document/SaveDocumentButton.tsx'
import { useDocumentState } from '@document/useDocumentState.ts'
import { FindBar } from '@find/FindBar.tsx'
import { DocumentMenu } from '@shell/DocumentMenu.tsx'
import type { LeftPanel } from '@shell/PanelStore.ts'
import { usePanelState } from '@shell/usePanelState.ts'

const TABS: { panel: LeftPanel; label: string }[] = [
  { panel: 'tools', label: 'All tools' },
  { panel: 'edit', label: 'Edit' },
]

export function AppHeader() {
  const { panels } = useApplication()
  const { left } = usePanelState()
  const document = useDocumentState()
  return (
    <header className="app-header">
      <div className="brand">
        <span className="brand-mark" aria-hidden="true">
          T
        </span>
        <span className="brand-name">Tintenfisch</span>
      </div>
      {document.phase === 'ready' && (
        <nav className="header-tabs" aria-label="Tool panels">
          {TABS.map(({ panel, label }) => (
            <button key={panel} type="button" className={`header-tab${left === panel ? ' header-tab-active' : ''}`} aria-pressed={left === panel} onClick={() => panels.toggleLeft(panel)}>
              {label}
            </button>
          ))}
        </nav>
      )}
      <div className="header-document">
        <DocumentTitle />
        <DocumentMenu />
      </div>
      <div className="header-actions">
        <FindBar />
        <OpenDocumentButton />
        <SaveDocumentButton />
      </div>
    </header>
  )
}
