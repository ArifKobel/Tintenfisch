import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { EditPanel } from '@editing/EditPanel.tsx'
import { PanelTabs } from '@shell/PanelTabs.tsx'
import { usePanelState } from '@shell/usePanelState.ts'
import { AllToolsPanel } from '@tools/AllToolsPanel.tsx'
import { IconButton } from '@ui/IconButton.tsx'

export function LeftPanels() {
  const { panels } = useApplication()
  const { left, sheet } = usePanelState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  return (
    <div className={`left-dock${sheet ? ' left-dock-open' : ''}`}>
      <div className="sheet-header">
        <PanelTabs className="sheet-tabs" />
        <IconButton icon="close" label="Close tools" tooltip={null} onClick={() => panels.closeSheet()} />
      </div>
      {left === 'tools' ? <AllToolsPanel /> : <EditPanel />}
    </div>
  )
}
