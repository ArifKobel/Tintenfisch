import { useApplication } from '@app/ApplicationContext.ts'
import { DocumentTitle } from '@document/DocumentTitle.tsx'
import { SaveDocumentButton } from '@document/SaveDocumentButton.tsx'
import { useDocumentState } from '@document/useDocumentState.ts'
import { SaveStatus } from '@files/SaveStatus.tsx'
import { FindBar } from '@find/FindBar.tsx'
import { DocumentMenu } from '@shell/DocumentMenu.tsx'
import { GitHubStarLink } from '@shell/GitHubStarLink.tsx'
import { PanelTabs } from '@shell/PanelTabs.tsx'

export function AppHeader() {
  const { files } = useApplication()
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
      {document.phase === 'ready' && <PanelTabs className="header-tabs" />}
      <div className="header-document">
        <DocumentTitle />
        <SaveStatus />
        <DocumentMenu />
      </div>
      <div className="header-actions">
        <FindBar />
        <SaveDocumentButton />
        <GitHubStarLink />
      </div>
    </header>
  )
}
