import { ToolRow } from '@app/ToolRow.tsx'
import { Toolbar } from '@app/Toolbar.tsx'
import { CommentsPanel } from '@comments/CommentsPanel.tsx'
import { useCommentShortcuts } from '@comments/useCommentShortcuts.ts'
import { FieldPanel } from '@forms/FieldPanel.tsx'
import { useFieldShortcuts } from '@forms/useFieldShortcuts.ts'
import { useImageShortcuts } from '@images/useImageShortcuts.ts'
import { BookmarksPanel } from '@navigation/BookmarksPanel.tsx'
import { LinkDialog } from '@navigation/LinkDialog.tsx'
import { useLinkShortcuts } from '@navigation/useLinkShortcuts.ts'
import { PrintDialog } from '@printing/PrintDialog.tsx'
import { PrintSheet } from '@printing/PrintSheet.tsx'
import { usePrintShortcut } from '@printing/usePrintShortcut.ts'
import { ProtectionDialog } from '@protection/ProtectionDialog.tsx'
import { useEditingShortcuts } from '@editing/useEditingShortcuts.ts'
import { useCopySelection } from '@selection/useCopySelection.ts'
import { SignaturePad } from '@signing/SignaturePad.tsx'
import { SigningDialog } from '@signing/SigningDialog.tsx'
import { ThumbnailSidebar } from '@thumbnails/ThumbnailSidebar.tsx'
import { DocumentView } from '@viewer/DocumentView.tsx'

export function App() {
  useCopySelection()
  useEditingShortcuts()
  useCommentShortcuts()
  useFieldShortcuts()
  useImageShortcuts()
  usePrintShortcut()
  useLinkShortcuts()
  return (
    <main className="app">
      <Toolbar />
      <ToolRow />
      <div className="workspace">
        <ThumbnailSidebar />
        <BookmarksPanel />
        <DocumentView />
        <CommentsPanel />
        <FieldPanel />
      </div>
      <SignaturePad />
      <SigningDialog />
      <PrintDialog />
      <LinkDialog />
      <ProtectionDialog />
      <PrintSheet />
    </main>
  )
}
