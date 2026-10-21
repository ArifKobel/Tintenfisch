import type { NewFieldKind, Rgb } from '@/types/protocol.ts'
import { TOOL_CATALOG } from '@/components/tools/toolCatalog.ts'
import { useWorkspaceStore, DRAWING_TOOLS, EDITING_TOOLS, MARKUP_COLORS, STAMP_LABELS } from '@/stores/workspaceStore.ts'
import { Select } from '@/components/ui/Field.tsx'
import { Icon } from '@/components/ui/Icon.tsx'
import { Swatch } from '@/components/ui/Swatch.tsx'

const FIELD_KINDS: { kind: NewFieldKind; label: string }[] = [
  { kind: 'text', label: 'Text field' },
  { kind: 'multiline', label: 'Text area' },
  { kind: 'checkbox', label: 'Checkbox' },
  { kind: 'radio', label: 'Radio button' },
  { kind: 'combo', label: 'Dropdown' },
  { kind: 'list', label: 'List box' },
]

const COLOR_NAMES = ['Red', 'Blue', 'Green', 'Yellow']

const css = ([r, g, b]: Rgb) => `rgb(${r * 255} ${g * 255} ${b * 255})`

export function ToolOptions() {
  const workspace = useWorkspaceStore()
  const { tool, stampLabel, fieldKind } = workspace
  if (tool === 'select' || EDITING_TOOLS.includes(tool)) return null
  const colored = tool === 'highlight' || (DRAWING_TOOLS.includes(tool) && tool !== 'redactArea')
  const color = tool === 'highlight' ? workspace.highlightColor : workspace.color
  return (
    <div className="flex min-h-10 items-center gap-2" role="group" aria-label={`${TOOL_CATALOG[tool].label} options`}>
      <span className="flex items-center gap-2 font-bold text-ink-strong">
        <Icon name={TOOL_CATALOG[tool].icon} />
        {TOOL_CATALOG[tool].label}
      </span>
      {colored && (
        <div className="flex items-center gap-1.5" role="radiogroup" aria-label="Color">
          {MARKUP_COLORS.map((swatch, index) => (
            <Swatch
              key={css(swatch)}
              color={css(swatch)}
              chosen={swatch === color}
              role="radio"
              aria-checked={swatch === color}
              aria-label={COLOR_NAMES[index]}
              onClick={() => workspace.setColor(swatch)}
            />
          ))}
        </div>
      )}
      {tool === 'form' && (
        <Select compact value={fieldKind} onChange={(event) => workspace.setFieldKind(event.target.value as NewFieldKind)} aria-label="Field kind">
          {FIELD_KINDS.map((entry) => (
            <option key={entry.kind} value={entry.kind}>
              {entry.label}
            </option>
          ))}
        </Select>
      )}
      {tool === 'stamp' && (
        <Select compact value={stampLabel} onChange={(event) => workspace.setStampLabel(event.target.value)} aria-label="Stamp">
          {STAMP_LABELS.map((label) => (
            <option key={label}>{label}</option>
          ))}
        </Select>
      )}
    </div>
  )
}
