import type { MouseEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import { FontPicker } from '@/components/editing/FontPicker.tsx'
import { useEditingState } from '@/hooks/useEditingState.ts'
import type { Rgb, TextStyle } from '@/types/protocol.ts'
import { Icon } from '@/components/ui/Icon.tsx'

const SIZES = [6, 8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72]

const COLORS: { color: Rgb; label: string }[] = [
  { color: [0, 0, 0], label: 'Black' },
  { color: [0.4, 0.4, 0.4], label: 'Gray' },
  { color: [0.8, 0.1, 0.1], label: 'Red' },
  { color: [0.9, 0.5, 0], label: 'Orange' },
  { color: [0.1, 0.55, 0.2], label: 'Green' },
  { color: [0.1, 0.3, 0.8], label: 'Blue' },
  { color: [0.5, 0.2, 0.7], label: 'Purple' },
]

const sameColor = (left: Rgb | null, right: Rgb) => left !== null && left.every((part, index) => Math.abs(part - right[index]) < 0.01)

function keepFocus(event: MouseEvent) {
  event.preventDefault()
}

export function FormatBar() {
  const { editing, fontCatalog, fontReplacements } = useApplication()
  const { active, fonts } = useEditingState()
  if (!active) return null
  const { look } = active
  const format = (style: TextStyle) => editing.edit((session) => session.formatted(style))
  const size = Math.round(look.size * 10) / 10
  const sizes = SIZES.includes(size) ? SIZES : [...SIZES, size].sort((left, right) => left - right)
  const documentFonts = editing.session?.origin ? [] : fonts

  async function restyle(style: { bold?: boolean; italic?: boolean }) {
    const font = look.font
    if (font?.kind !== 'catalog') return format(style)
    const family = (await fontCatalog.families()).find((candidate) => candidate.source === font.source && candidate.id === font.id)
    const choice = family && (await fontReplacements.catalogFont(family, (style.bold ?? look.bold) ? 700 : 400, style.italic ?? look.italic))
    if (choice) format({ ...style, font: choice })
  }
  return (
    <div className="format-bar" role="group" aria-label="Text format">
      <FontPicker look={look} fonts={documentFonts} onChoose={(font) => format({ font })} />
      <select className="field format-size" aria-label="Font size" value={size} onChange={(event) => format({ size: Number(event.target.value) })}>
        {sizes.map((choice) => (
          <option key={choice} value={choice}>
            {choice}
          </option>
        ))}
      </select>
      <button
        type="button"
        className={`icon-button${look.bold ? ' icon-button-toggled' : ''}`}
        aria-label="Bold"
        aria-pressed={look.bold}
        onMouseDown={keepFocus}
        onClick={() => void restyle({ bold: !look.bold })}
      >
        <Icon name="text-bold" />
      </button>
      <button
        type="button"
        className={`icon-button${look.italic ? ' icon-button-toggled' : ''}`}
        aria-label="Italic"
        aria-pressed={look.italic}
        onMouseDown={keepFocus}
        onClick={() => void restyle({ italic: !look.italic })}
      >
        <Icon name="text-italic" />
      </button>
      <div className="swatches" role="group" aria-label="Text color">
        {COLORS.map(({ color, label }) => (
          <button
            key={label}
            type="button"
            className={`swatch${sameColor(look.color, color) ? ' swatch-chosen' : ''}`}
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
