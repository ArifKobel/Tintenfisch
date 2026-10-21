import type { MouseEvent } from 'react'
import { FontPicker } from '@/components/editing/FontPicker.tsx'
import { catalogFamilies } from '@/services/fontCatalog.ts'
import { useEditingStore } from '@/stores/editingStore.ts'
import { useFontStore } from '@/stores/fontStore.ts'
import type { Rgb, TextStyle } from '@/types/protocol.ts'
import { Select } from '@/components/ui/Field.tsx'
import { Icon } from '@/components/ui/Icon.tsx'
import { iconButtonClass } from '@/components/ui/controlStyles.ts'
import { Swatch } from '@/components/ui/Swatch.tsx'

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
  const editing = useEditingStore()
  const { active, fonts } = editing
  const catalogFont = useFontStore((state) => state.catalogFont)
  if (!active) return null
  const { look } = active
  const format = (style: TextStyle) => editing.edit((session) => session.formatted(style))
  const size = Math.round(look.size * 10) / 10
  const sizes = SIZES.includes(size) ? SIZES : [...SIZES, size].sort((left, right) => left - right)
  const documentFonts = editing.session()?.origin ? [] : fonts

  async function restyle(style: { bold?: boolean; italic?: boolean }) {
    const font = look.font
    if (font?.kind !== 'catalog') return format(style)
    const family = (await catalogFamilies()).find((candidate) => candidate.source === font.source && candidate.id === font.id)
    const choice = family && (await catalogFont(family, (style.bold ?? look.bold) ? 700 : 400, style.italic ?? look.italic))
    if (choice) format({ ...style, font: choice })
  }
  return (
    <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Text format" data-keeps-edit>
      <FontPicker look={look} fonts={documentFonts} onChoose={(font) => format({ font })} />
      <Select className="mr-auto w-[88px]" aria-label="Font size" value={size} onChange={(event) => format({ size: Number(event.target.value) })}>
        {sizes.map((choice) => (
          <option key={choice} value={choice}>
            {choice}
          </option>
        ))}
      </Select>
      <button
        type="button"
        className={iconButtonClass(look.bold ? 'toggled' : null)}
        aria-label="Bold"
        aria-pressed={look.bold}
        onMouseDown={keepFocus}
        onClick={() => void restyle({ bold: !look.bold })}
      >
        <Icon name="text-bold" />
      </button>
      <button
        type="button"
        className={iconButtonClass(look.italic ? 'toggled' : null)}
        aria-label="Italic"
        aria-pressed={look.italic}
        onMouseDown={keepFocus}
        onClick={() => void restyle({ italic: !look.italic })}
      >
        <Icon name="text-italic" />
      </button>
      <div className="flex w-full items-center gap-1.5" role="group" aria-label="Text color">
        {COLORS.map(({ color, label }) => (
          <Swatch
            key={label}
            color={`rgb(${color.map((part) => Math.round(part * 255)).join(' ')})`}
            chosen={sameColor(look.color, color)}
            aria-label={label}
            aria-pressed={sameColor(look.color, color)}
            onMouseDown={keepFocus}
            onClick={() => format({ color })}
          />
        ))}
      </div>
    </div>
  )
}
