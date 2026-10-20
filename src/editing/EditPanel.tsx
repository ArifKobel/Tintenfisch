import { useApplication } from '@app/ApplicationContext.ts'
import { FormatBar } from '@editing/FormatBar.tsx'
import { useEditingState } from '@editing/useEditingState.ts'
import { TOOL_CATALOG } from '@tools/ToolCatalog.ts'
import type { Tool } from '@tools/ToolStore.ts'
import { useToolState } from '@tools/useToolState.ts'
import { PanelItem } from '@ui/PanelItem.tsx'
import { SidePanel } from '@ui/SidePanel.tsx'

const CONTENT_TOOLS: Tool[] = ['addText', 'addImage', 'link']

export function EditPanel() {
  const { panels, tools } = useApplication()
  const { tool } = useToolState()
  const { active } = useEditingState()
  const choose = (chosen: Tool) => {
    tools.choose(chosen)
    panels.closeSheet()
  }
  const pick = (chosen: Tool) => choose(tool === chosen ? 'edit' : chosen)
  return (
    <SidePanel title="Edit" side="left">
      <section className="panel-section">
        <h3 className="section-label">Format text</h3>
        {active ? <FormatBar /> : <p className="hint">Click into text on the page to change it and its format.</p>}
      </section>
      <section className="panel-section">
        <h3 className="section-label">Add content</h3>
        {CONTENT_TOOLS.map((entry) => (
          <PanelItem key={entry} icon={TOOL_CATALOG[entry].icon} label={TOOL_CATALOG[entry].label} chosen={tool === entry} onClick={() => pick(entry)} />
        ))}
      </section>
      <section className="panel-section">
        <h3 className="section-label">More options</h3>
        <PanelItem icon="view-grid" label="Organize pages" onClick={() => panels.showRight('pages')} />
        <PanelItem icon={TOOL_CATALOG.redactText.icon} label={TOOL_CATALOG.redactText.label} onClick={() => choose('redactText')} />
        <PanelItem icon={TOOL_CATALOG.redactArea.icon} label={TOOL_CATALOG.redactArea.label} onClick={() => choose('redactArea')} />
      </section>
    </SidePanel>
  )
}
