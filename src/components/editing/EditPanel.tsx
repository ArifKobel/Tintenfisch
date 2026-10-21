import { FormatBar } from '@/components/editing/FormatBar.tsx'
import { TOOL_CATALOG } from '@/components/tools/toolCatalog.ts'
import { PanelItem } from '@/components/ui/PanelItem.tsx'
import { PanelSection, SidePanel } from '@/components/ui/SidePanel.tsx'
import { Hint, SectionLabel } from '@/components/ui/Text.tsx'
import { useEditingStore } from '@/stores/editingStore.ts'
import { useWorkspaceStore, type Tool } from '@/stores/workspaceStore.ts'

const CONTENT_TOOLS: Tool[] = ['addText', 'addImage', 'link']

export function EditPanel() {
  const workspace = useWorkspaceStore()
  const { tool } = workspace
  const active = useEditingStore((state) => state.active)
  const choose = (chosen: Tool) => {
    workspace.choose(chosen)
    workspace.closeSheet()
  }
  const pick = (chosen: Tool) => choose(tool === chosen ? 'edit' : chosen)
  return (
    <SidePanel title="Edit" side="left">
      <PanelSection>
        <SectionLabel>Format text</SectionLabel>
        {active ? <FormatBar /> : <Hint>Click into text on the page to change it and its format.</Hint>}
      </PanelSection>
      <PanelSection>
        <SectionLabel>Add content</SectionLabel>
        {CONTENT_TOOLS.map((entry) => (
          <PanelItem key={entry} icon={TOOL_CATALOG[entry].icon} label={TOOL_CATALOG[entry].label} chosen={tool === entry} onClick={() => pick(entry)} />
        ))}
      </PanelSection>
      <PanelSection>
        <SectionLabel>More options</SectionLabel>
        <PanelItem icon="view-grid" label="Organize pages" onClick={() => workspace.showRight('pages')} />
        <PanelItem icon={TOOL_CATALOG.redactText.icon} label={TOOL_CATALOG.redactText.label} onClick={() => choose('redactText')} />
        <PanelItem icon={TOOL_CATALOG.redactArea.icon} label={TOOL_CATALOG.redactArea.label} onClick={() => choose('redactArea')} />
      </PanelSection>
    </SidePanel>
  )
}
