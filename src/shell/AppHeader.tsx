import { useApplication } from '@app/ApplicationContext.ts'
import { DocumentTitle } from '@document/DocumentTitle.tsx'
import { OpenDocumentButton } from '@document/OpenDocumentButton.tsx'
import { SaveDocumentButton } from '@document/SaveDocumentButton.tsx'
import { useDocumentState } from '@document/useDocumentState.ts'
import { SaveStatus } from '@files/SaveStatus.tsx'
import { FindBar } from '@find/FindBar.tsx'
import { DocumentMenu } from '@shell/DocumentMenu.tsx'
import type { LeftPanel } from '@shell/PanelStore.ts'
import { usePanelState } from '@shell/usePanelState.ts'

const TABS: { panel: LeftPanel; label: string }[] = [
  { panel: 'tools', label: 'All tools' },
  { panel: 'edit', label: 'Edit' },
]

export function AppHeader() {
  const { files, panels } = useApplication()
  const { left } = usePanelState()
  const document = useDocumentState()
  return (
    <header className="app-header">
      <a
        className="brand"
        href="/"
        onClick={(event) => {
          event.preventDefault()
          files.goHome()
        }}
      >
        <img className="brand-mark" src="/brand/mark.png" alt="" width={28} height={28} />
        <span className="brand-name">Tintenfisch</span>
      </a>
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
        <SaveStatus />
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
