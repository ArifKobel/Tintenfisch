import { EditPanel } from '@/components/editing/EditPanel.tsx'
import { PanelTabs } from '@/components/layout/PanelTabs.tsx'
import { AllToolsPanel } from '@/components/tools/AllToolsPanel.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function LeftDock() {
  const { left, sheet, closeSheet } = useWorkspaceStore()
  const phase = useDocumentStore((state) => state.document.phase)
  if (phase !== 'ready') return null
  return (
    <div className={`contents ${sheet ? 'max-md:fixed max-md:inset-x-0 max-md:bottom-0 max-md:z-26 max-md:flex max-md:max-h-[50vh] max-md:flex-col max-md:rounded-t-xl max-md:bg-surface max-md:shadow-[0_-4px_16px_rgb(0_0_0/0.15)]' : 'max-md:hidden'}`}>
      <div className="hidden h-12 flex-none items-center justify-between border-b border-divider pr-3 pl-5 max-md:flex">
        <PanelTabs className="" />
        <IconButton icon="close" label="Close tools" tooltip={null} onClick={closeSheet} />
      </div>
      {left === 'tools' ? <AllToolsPanel /> : <EditPanel />}
    </div>
  )
}
