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
    <div className={`left-dock${sheet ? ' left-dock-open' : ''}`}>
      <div className="sheet-header">
        <PanelTabs className="sheet-tabs" />
        <IconButton icon="close" label="Close tools" tooltip={null} onClick={closeSheet} />
      </div>
      {left === 'tools' ? <AllToolsPanel /> : <EditPanel />}
    </div>
  )
}
