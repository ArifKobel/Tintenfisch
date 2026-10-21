import { useCommentShortcuts } from '@/hooks/useCommentShortcuts.ts'
import { useEditingShortcuts } from '@/hooks/useEditingShortcuts.ts'
import { FileDropZone } from '@/components/home/FileDropZone.tsx'
import { FontConsentDialog } from '@/components/editing/FontConsentDialog.tsx'
import { useFieldShortcuts } from '@/hooks/useFieldShortcuts.ts'
import { useImageShortcuts } from '@/hooks/useImageShortcuts.ts'
import { LinkDialog } from '@/components/links/LinkDialog.tsx'
import { useLinkShortcuts } from '@/hooks/useLinkShortcuts.ts'
import { PrintDialog } from '@/components/printing/PrintDialog.tsx'
import { PrintSheet } from '@/components/printing/PrintSheet.tsx'
import { usePrintShortcut } from '@/hooks/usePrintShortcut.ts'
import { ProtectionDialog } from '@/components/protection/ProtectionDialog.tsx'
import { useCopySelection } from '@/hooks/useCopySelection.ts'
import { AppHeader } from '@/components/layout/AppHeader.tsx'
import { ContextBar } from '@/components/layout/ContextBar.tsx'
import { LeftPanels } from '@/components/layout/LeftDock.tsx'
import { RightPanels } from '@/components/layout/RightDock.tsx'
import { RightRail } from '@/components/layout/RightRail.tsx'
import { Toasts } from '@/components/layout/Toasts.tsx'
import { SignaturePad } from '@/components/signing/SignaturePad.tsx'
import { SignatureStatusList } from '@/components/signing/SignatureStatusList.tsx'
import { SigningDialog } from '@/components/signing/SigningDialog.tsx'
import { QuickToolbar } from '@/components/layout/QuickToolbar.tsx'
import { DocumentView } from '@/components/viewer/DocumentView.tsx'

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
      <FontConsentDialog />
      <PrintSheet />
      <FileDropZone />
    </main>
  )
}
