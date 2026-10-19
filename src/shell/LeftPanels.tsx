import { useDocumentState } from '@document/useDocumentState.ts'
import { EditPanel } from '@editing/EditPanel.tsx'
import { usePanelState } from '@shell/usePanelState.ts'
import { AllToolsPanel } from '@tools/AllToolsPanel.tsx'

export function LeftPanels() {
  const { left } = usePanelState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  if (left === 'tools') return <AllToolsPanel />
  if (left === 'edit') return <EditPanel />
  return null
}
