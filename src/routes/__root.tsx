import { createRootRoute, Outlet } from '@tanstack/react-router'
import { FontConsentDialog } from '@/components/editing/FontConsentDialog.tsx'
import { FileDropZone } from '@/components/home/FileDropZone.tsx'
import { AppHeader } from '@/components/layout/AppHeader.tsx'
import { ContextBar } from '@/components/layout/ContextBar.tsx'
import { LeftDock } from '@/components/layout/LeftDock.tsx'
import { QuickToolbar } from '@/components/layout/QuickToolbar.tsx'
import { RightDock } from '@/components/layout/RightDock.tsx'
import { RightRail } from '@/components/layout/RightRail.tsx'
import { Toasts } from '@/components/layout/Toasts.tsx'
import { LinkDialog } from '@/components/links/LinkDialog.tsx'
import { PrintDialog } from '@/components/printing/PrintDialog.tsx'
import { PrintSheet } from '@/components/printing/PrintSheet.tsx'
import { ProtectionDialog } from '@/components/protection/ProtectionDialog.tsx'
import { SignaturePad } from '@/components/signing/SignaturePad.tsx'
import { SignatureStatusList } from '@/components/signing/SignatureStatusList.tsx'
import { SigningDialog } from '@/components/signing/SigningDialog.tsx'
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts.ts'

export const Route = createRootRoute({ component: Shell })

function Shell() {
  useKeyboardShortcuts()
  return (
    <main className="flex h-full flex-col">
      <AppHeader />
      <div className="flex min-h-0 flex-1">
        <LeftDock />
        <div className="flex min-w-0 flex-1 flex-col bg-canvas">
          <SignatureStatusList />
          <div className="relative flex min-h-0 flex-1 max-md:flex-col">
            <QuickToolbar />
            <Outlet />
            <ContextBar />
            <Toasts />
          </div>
        </div>
        <RightDock />
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
