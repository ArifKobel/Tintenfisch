import { useCommentShortcuts } from '@comments/useCommentShortcuts.ts'
import { useEditingShortcuts } from '@editing/useEditingShortcuts.ts'
import { FileDropZone } from '@files/FileDropZone.tsx'
import { useFieldShortcuts } from '@forms/useFieldShortcuts.ts'
import { useImageShortcuts } from '@images/useImageShortcuts.ts'
import { LinkDialog } from '@navigation/LinkDialog.tsx'
import { useLinkShortcuts } from '@navigation/useLinkShortcuts.ts'
import { PrintDialog } from '@printing/PrintDialog.tsx'
import { PrintSheet } from '@printing/PrintSheet.tsx'
import { usePrintShortcut } from '@printing/usePrintShortcut.ts'
import { ProtectionDialog } from '@protection/ProtectionDialog.tsx'
import { useCopySelection } from '@selection/useCopySelection.ts'
import { AppHeader } from '@shell/AppHeader.tsx'
import { ContextBar } from '@shell/ContextBar.tsx'
import { LeftPanels } from '@shell/LeftPanels.tsx'
import { RightPanels } from '@shell/RightPanels.tsx'
import { RightRail } from '@shell/RightRail.tsx'
import { Toasts } from '@shell/Toasts.tsx'
import { SignaturePad } from '@signing/SignaturePad.tsx'
import { SignatureStatusList } from '@signing/SignatureStatusList.tsx'
import { SigningDialog } from '@signing/SigningDialog.tsx'
import { QuickToolbar } from '@tools/QuickToolbar.tsx'
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
      <AppHeader />
      <div className="workspace">
        <LeftPanels />
        <div className="document-area">
          <SignatureStatusList />
          <div className="canvas">
            <QuickToolbar />
            <DocumentView />
            <ContextBar />
            <Toasts />
          </div>
        </div>
        <RightPanels />
        <RightRail />
      </div>
      <SignaturePad />
      <SigningDialog />
      <PrintDialog />
      <LinkDialog />
      <ProtectionDialog />
      <PrintSheet />
      <FileDropZone />
    </main>
  )
}
