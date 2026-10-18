import { CommentsButton } from '@comments/CommentsButton.tsx'
import { BookmarksButton } from '@navigation/BookmarksButton.tsx'
import { DocumentStatus } from '@document/DocumentStatus.tsx'
import { FindBar } from '@find/FindBar.tsx'
import { OpenDocumentButton } from '@document/OpenDocumentButton.tsx'
import { PrintButton } from '@printing/PrintButton.tsx'
import { ProtectButton } from '@protection/ProtectButton.tsx'
import { SaveDocumentButton } from '@document/SaveDocumentButton.tsx'
import { ZoomControls } from '@viewer/ZoomControls.tsx'

export function Toolbar() {
  return (
    <header className="toolbar">
      <OpenDocumentButton />
      <SaveDocumentButton />
      <PrintButton />
      <ProtectButton />
      <ZoomControls />
      <FindBar />
      <BookmarksButton />
      <CommentsButton />
      <DocumentStatus />
    </header>
  )
}
