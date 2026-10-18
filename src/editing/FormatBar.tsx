import type { MouseEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { useEditingState } from '@editing/useEditingState.ts'
import type { BundledFamily, FontChoice, Rgb, TextStyle } from '@engine/protocol.ts'

const SIZES = [6, 8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72]

const FAMILIES: { family: BundledFamily; label: string }[] = [
  { family: 'sans', label: 'Sans' },
  { family: 'serif', label: 'Serif' },
  { family: 'mono', label: 'Mono' },
]

const COLORS: { color: Rgb; label: string }[] = [
  { color: [0, 0, 0], label: 'Black' },
  { color: [0.4, 0.4, 0.4], label: 'Gray' },
  { color: [0.8, 0.1, 0.1], label: 'Red' },
  { color: [0.9, 0.5, 0], label: 'Orange' },
  { color: [0.1, 0.55, 0.2], label: 'Green' },
  { color: [0.1, 0.3, 0.8], label: 'Blue' },
  { color: [0.5, 0.2, 0.7], label: 'Purple' },
]

const valueOf = (font: FontChoice | null): string => (font === null ? '' : font.kind === 'document' ? `document:${font.name}` : `bundled:${font.family}`)

function choiceOf(value: string): FontChoice {
  const [kind, name] = value.split(/:(.*)/)
  return kind === 'document' ? { kind: 'document', name } : { kind: 'bundled', family: name as BundledFamily }
}

const sameColor = (left: Rgb | null, right: Rgb) => left !== null && left.every((part, index) => Math.abs(part - right[index]) < 0.01)

function keepFocus(event: MouseEvent) {
  event.preventDefault()
}

export function FormatBar() {
  const { editing } = useApplication()
  const { active, fonts } = useEditingState()
  if (!active) return null
  const { look } = active
  const format = (style: TextStyle) => editing.edit((session) => session.formatted(style))
  const size = Math.round(look.size * 10) / 10
  const sizes = SIZES.includes(size) ? SIZES : [...SIZES, size].sort((left, right) => left - right)
  const documentFonts = editing.session?.origin ? [] : fonts
  return (
    <div className="format-bar" role="group" aria-label="Text format">
      <select className="format-font" aria-label="Font" value={valueOf(look.font)} onChange={(event) => format({ font: choiceOf(event.target.value) })}>
        {look.font === null && <option value="">Font of the text</option>}
        {documentFonts.map((font) => (
          <option key={font.name} value={`document:${font.name}`}>
            {font.font || font.name}
          </option>
        ))}
        {FAMILIES.map(({ family, label }) => (
          <option key={family} value={`bundled:${family}`}>
            {label}
          </option>
        ))}
      </select>
      <select className="format-size" aria-label="Font size" value={size} onChange={(event) => format({ size: Number(event.target.value) })}>
        {sizes.map((choice) => (
          <option key={choice} value={choice}>
            {choice}
          </option>
        ))}
      </select>
      <button
        type="button"
        className={`button button-small format-bold${look.bold ? ' button-active' : ''}`}
        aria-label="Bold"
        aria-pressed={look.bold}
        onMouseDown={keepFocus}
        onClick={() => format({ bold: !look.bold })}
      >
        B
      </button>
      <button
        type="button"
        className={`button button-small format-italic${look.italic ? ' button-active' : ''}`}
        aria-label="Italic"
        aria-pressed={look.italic}
        onMouseDown={keepFocus}
        onClick={() => format({ italic: !look.italic })}
      >
        I
      </button>
      <div className="format-colors" role="group" aria-label="Text color">
        {COLORS.map(({ color, label }) => (
          <button
            key={label}
            type="button"
            className={`format-color${sameColor(look.color, color) ? ' format-color-chosen' : ''}`}
            aria-label={label}
            aria-pressed={sameColor(look.color, color)}
            style={{ background: `rgb(${color.map((part) => Math.round(part * 255)).join(' ')})` }}
            onMouseDown={keepFocus}
            onClick={() => format({ color })}
          />
        ))}
      </div>
    </div>
  )
}
