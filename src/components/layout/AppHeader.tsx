import { Link } from '@tanstack/react-router'
import { DocumentTitle } from '@/components/document/DocumentTitle.tsx'
import { SaveDocumentButton } from '@/components/document/SaveDocumentButton.tsx'
import { SaveStatus } from '@/components/document/SaveStatus.tsx'
import { DocumentMenu } from '@/components/layout/DocumentMenu.tsx'
import { GitHubStarLink } from '@/components/layout/GitHubStarLink.tsx'
import { PanelTabs } from '@/components/layout/PanelTabs.tsx'
import { FindBar } from '@/components/text/FindBar.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'

export function AppHeader() {
  const document = useDocumentStore((state) => state.document)
  return (
    <header className="app-header">
      <Link className="brand" to="/">
        <img className="brand-mark" src="/brand/mark.png" alt="" width={28} height={28} />
        <span className="brand-name">Tintenfisch</span>
      </Link>
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
