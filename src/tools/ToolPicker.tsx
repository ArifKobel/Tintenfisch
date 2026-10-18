import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import type { NewFieldKind, Rgb } from '@engine/protocol.ts'
import { MARKUP_COLORS, STAMP_LABELS, type Tool } from '@tools/ToolStore.ts'
import { useToolState } from '@tools/useToolState.ts'

const TOOLS: { tool: Tool; label: string }[] = [
  { tool: 'select', label: 'Select' },
  { tool: 'edit', label: 'Edit text' },
  { tool: 'addText', label: 'Add text' },
  { tool: 'addImage', label: 'Add image' },
  { tool: 'highlight', label: 'Highlight' },
  { tool: 'note', label: 'Note' },
  { tool: 'textBox', label: 'Text box' },
  { tool: 'ink', label: 'Draw' },
  { tool: 'rectangle', label: 'Rectangle' },
  { tool: 'ellipse', label: 'Ellipse' },
  { tool: 'arrow', label: 'Arrow' },
  { tool: 'stamp', label: 'Stamp' },
  { tool: 'redactText', label: 'Redact text' },
  { tool: 'redactArea', label: 'Redact area' },
  { tool: 'form', label: 'Form' },
  { tool: 'link', label: 'Link' },
  { tool: 'sign', label: 'Sign' },
]

const FIELD_KINDS: { kind: NewFieldKind; label: string }[] = [
  { kind: 'text', label: 'Text field' },
  { kind: 'multiline', label: 'Text area' },
  { kind: 'checkbox', label: 'Checkbox' },
  { kind: 'radio', label: 'Radio button' },
  { kind: 'combo', label: 'Dropdown' },
  { kind: 'list', label: 'List box' },
]

const css = ([r, g, b]: Rgb) => `rgb(${r * 255} ${g * 255} ${b * 255})`

export function ToolPicker() {
  const { tools, signing, digitalSignatures } = useApplication()
  const state = useToolState()
  const { tool, stampLabel, fieldKind } = state
  const color = tool === 'highlight' ? state.highlightColor : state.color
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  return (
    <div className="tool-picker" role="toolbar" aria-label="Tools">
      <div className="tool-group">
        {TOOLS.map((entry) => (
          <button
            key={entry.tool}
            type="button"
            className={`button button-small${entry.tool === tool ? ' button-active' : ''}`}
            aria-pressed={entry.tool === tool}
            onClick={() => tools.choose(entry.tool)}
          >
            {entry.label}
          </button>
        ))}
      </div>
      {tool === 'sign' && (
        <button type="button" className="button button-small" onClick={() => signing.openPad()}>
          New signature
        </button>
      )}
      {tool === 'sign' && (
        <button type="button" className="button button-small" onClick={() => digitalSignatures.openDialog()}>
          Sign with certificate
        </button>
      )}
      {tool !== 'select' && tool !== 'edit' && tool !== 'addText' && tool !== 'addImage' && tool !== 'form' && tool !== 'link' && tool !== 'sign' && (
        <div className="tool-group" role="radiogroup" aria-label="Color">
          {MARKUP_COLORS.map((swatch) => (
            <button
              key={css(swatch)}
              type="button"
              role="radio"
              aria-checked={swatch === color}
              aria-label={css(swatch)}
              className={`color-swatch${swatch === color ? ' color-swatch-chosen' : ''}`}
              style={{ background: css(swatch) }}
              onClick={() => tools.setColor(swatch)}
            />
          ))}
        </div>
      )}
      {tool === 'form' && (
        <select className="stamp-label" value={fieldKind} onChange={(event) => tools.setFieldKind(event.target.value as NewFieldKind)} aria-label="Field kind">
          {FIELD_KINDS.map((entry) => (
            <option key={entry.kind} value={entry.kind}>
              {entry.label}
            </option>
          ))}
        </select>
      )}
      {tool === 'stamp' && (
        <select className="stamp-label" value={stampLabel} onChange={(event) => tools.setStampLabel(event.target.value)} aria-label="Stamp">
          {STAMP_LABELS.map((label) => (
            <option key={label}>{label}</option>
          ))}
        </select>
      )}
    </div>
  )
}
