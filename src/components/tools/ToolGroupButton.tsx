import { TOOL_CATALOG } from '@/components/tools/toolCatalog.ts'
import { useWorkspaceStore, type Tool } from '@/stores/workspaceStore.ts'
import { Icon } from '@/components/ui/Icon.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { MenuButton, MenuItem } from '@/components/ui/Menu.tsx'

export function ToolGroupButton({ label, tools: group, current }: { label: string; tools: readonly Tool[]; current: Tool }) {
  const tools = useWorkspaceStore()
  const active = group.includes(current)
  const shown = TOOL_CATALOG[active ? current : group[0]]
  if (group.length === 1) {
    return <IconButton icon={shown.icon} label={label} tooltip="right" look={active ? 'selected' : null} pressed={active} onClick={() => tools.choose(group[0])} />
  }
  return (
    <MenuButton icon={shown.icon} label={label} tooltip="right" look={active ? 'selected' : null} opens="right">
      {(close) =>
        group.map((tool) => (
          <MenuItem
            key={tool}
            role="menuitemradio"
            aria-checked={tool === current}
            chosen={tool === current}
            onClick={() => {
              tools.choose(tool)
              close()
            }}
          >
            <Icon name={TOOL_CATALOG[tool].icon} />
            {TOOL_CATALOG[tool].label}
          </MenuItem>
        ))
      }
    </MenuButton>
  )
}
