import { Link } from '@tanstack/react-router'
import { isTauri } from '@tauri-apps/api/core'
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
    <header className="relative z-20 flex h-14 flex-none items-center gap-4 bg-surface px-4 shadow-[0_1px_0_var(--color-divider)] max-md:gap-2 max-md:px-3">
      <Link className="flex items-center gap-2.5 border-r border-separator pr-4 text-inherit no-underline max-md:border-r-0 max-md:pr-0" to="/">
        <img className="block size-7 rounded-md" src="/brand/mark.png" alt="" width={28} height={28} />
        <span className="text-[15px] font-bold text-ink-strong max-md:hidden">Tintenfisch</span>
      </Link>
      {document.phase === 'ready' && <PanelTabs className="max-md:hidden" />}
      <div className="flex min-w-0 flex-1 items-center justify-center gap-2 max-md:justify-start max-md:gap-1">
        <DocumentTitle />
        <SaveStatus />
        <DocumentMenu />
      </div>
      <div className="flex items-center gap-2 max-md:gap-1">
        <FindBar />
        <SaveDocumentButton />
        {!isTauri() && <GitHubStarLink />}
      </div>
    </header>
  )
}
