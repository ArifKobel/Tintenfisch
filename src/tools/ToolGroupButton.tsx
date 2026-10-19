import { useApplication } from '@app/ApplicationContext.ts'
import { TOOL_CATALOG } from '@tools/ToolCatalog.ts'
import type { Tool } from '@tools/ToolStore.ts'
import { Icon } from '@ui/Icon.tsx'
import { IconButton } from '@ui/IconButton.tsx'
import { MenuButton } from '@ui/MenuButton.tsx'

export function ToolGroupButton({ label, tools: group, current }: { label: string; tools: readonly Tool[]; current: Tool }) {
  const { tools } = useApplication()
  const active = group.includes(current)
  const shown = TOOL_CATALOG[active ? current : group[0]]
  if (group.length === 1) {
    return <IconButton icon={shown.icon} label={label} tooltip="right" look={active ? 'selected' : null} pressed={active} onClick={() => tools.choose(group[0])} />
  }
  return (
    <MenuButton icon={shown.icon} label={label} tooltip="right" look={active ? 'selected' : null} opens="right">
      {(close) =>
        group.map((tool) => (
          <button
            key={tool}
            type="button"
            role="menuitemradio"
            aria-checked={tool === current}
            className={`menu-item${tool === current ? ' menu-item-chosen' : ''}`}
            onClick={() => {
              tools.choose(tool)
              close()
            }}
          >
            <Icon name={TOOL_CATALOG[tool].icon} />
            {TOOL_CATALOG[tool].label}
          </button>
        ))
      }
    </MenuButton>
  )
}
