import { useApplication } from '@/app/ApplicationContext.ts'
import type { NewFieldKind, Rgb } from '@/types/protocol.ts'
import { TOOL_CATALOG } from '@/lib/toolCatalog.ts'
import { DRAWING_TOOLS, EDITING_TOOLS, MARKUP_COLORS, STAMP_LABELS } from '@/stores/ToolStore.ts'
import { useToolState } from '@/hooks/useToolState.ts'
import { Icon } from '@/components/ui/Icon.tsx'

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
  const { tools } = useApplication()
  const state = useToolState()
  const { tool, stampLabel, fieldKind } = state
  if (tool === 'select' || EDITING_TOOLS.includes(tool)) return null
  const colored = tool === 'highlight' || (DRAWING_TOOLS.includes(tool) && tool !== 'redactArea')
  const color = tool === 'highlight' ? state.highlightColor : state.color
  return (
    <div className="tool-options" role="group" aria-label={`${TOOL_CATALOG[tool].label} options`}>
      <span className="tool-options-name">
        <Icon name={TOOL_CATALOG[tool].icon} />
        {TOOL_CATALOG[tool].label}
      </span>
      {colored && (
        <div className="swatches" role="radiogroup" aria-label="Color">
          {MARKUP_COLORS.map((swatch, index) => (
            <button
              key={css(swatch)}
              type="button"
              role="radio"
              aria-checked={swatch === color}
              aria-label={COLOR_NAMES[index]}
              className={`swatch${swatch === color ? ' swatch-chosen' : ''}`}
              style={{ background: css(swatch) }}
              onClick={() => tools.setColor(swatch)}
            />
          ))}
        </div>
      )}
      {tool === 'form' && (
        <select className="field field-compact" value={fieldKind} onChange={(event) => tools.setFieldKind(event.target.value as NewFieldKind)} aria-label="Field kind">
          {FIELD_KINDS.map((entry) => (
            <option key={entry.kind} value={entry.kind}>
              {entry.label}
            </option>
          ))}
        </select>
      )}
      {tool === 'stamp' && (
        <select className="field field-compact" value={stampLabel} onChange={(event) => tools.setStampLabel(event.target.value)} aria-label="Stamp">
          {STAMP_LABELS.map((label) => (
            <option key={label}>{label}</option>
          ))}
        </select>
      )}
    </div>
  )
}
