import { useApplication } from '@/app/ApplicationContext.ts'
import { DocumentTitle } from '@/components/document/DocumentTitle.tsx'
import { SaveDocumentButton } from '@/components/document/SaveDocumentButton.tsx'
import { useDocumentState } from '@/hooks/useDocumentState.ts'
import { SaveStatus } from '@/components/document/SaveStatus.tsx'
import { FindBar } from '@/components/text/FindBar.tsx'
import { DocumentMenu } from '@/components/layout/DocumentMenu.tsx'
import { GitHubStarLink } from '@/components/layout/GitHubStarLink.tsx'
import { PanelTabs } from '@/components/layout/PanelTabs.tsx'

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
