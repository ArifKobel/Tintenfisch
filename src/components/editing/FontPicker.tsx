import { useEffect, useId, useRef, useState, type KeyboardEvent } from 'react'
import type { TextLook } from '@/lib/EditSession.ts'
import { FontOptions } from '@/lib/FontOptions.ts'
import type { BundledFamily, FontChoice, PageFont } from '@/types/protocol.ts'
import { catalogFamilies, type CatalogFamily } from '@/services/fontCatalog.ts'
import { useFontStore } from '@/stores/fontStore.ts'
import { CHEVRON, FIELD } from '@/components/ui/controlStyles.ts'
import { Hint } from '@/components/ui/Text.tsx'

const BUNDLED: { family: BundledFamily; label: string }[] = [
  { family: 'sans', label: 'Sans' },
  { family: 'serif', label: 'Serif' },
  { family: 'mono', label: 'Mono' },
]

const SHOWN_PER_SOURCE = 50

const CATEGORY_LABELS: Record<string, string> = { sans: 'Sans', serif: 'Serif', mono: 'Mono', display: 'Display', handwriting: 'Handwriting', other: '' }

interface Entry {
  key: string
  label: string
  detail: string
  chosen: boolean
  choose: () => void
}

interface Section {
  title: string
  entries: Entry[]
  hidden: number
}

function weightOf(font: FontChoice | null, look: TextLook, fonts: readonly PageFont[]): number {
  if (font?.kind === 'catalog') return font.weight
  const own = font?.kind === 'document' ? fonts.find((candidate) => candidate.name === font.name)?.weight : null
  return own ?? (look.bold ? 700 : 400)
}

function labelOf(font: FontChoice | null, options: FontOptions): string {
  if (font === null) return 'Font of the text'
  if (font.kind === 'catalog') return font.family
  if (font.kind === 'bundled') return BUNDLED.find(({ family }) => family === font.family)?.label ?? font.family
  const shown = options.shownAs(font.name)
  return options.options.find((option) => option.name === shown)?.label ?? font.name
}

export function FontPicker({ look, fonts, onChoose }: { look: TextLook; fonts: readonly PageFont[]; onChoose: (font: FontChoice) => void }) {
  const catalogFont = useFontStore((state) => state.catalogFont)
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const [families, setFamilies] = useState<CatalogFamily[] | null>(null)
  const [loading, setLoading] = useState<string | null>(null)
  const [place, setPlace] = useState<{ top: number; left: number; width: number } | null>(null)
  const root = useRef<HTMLDivElement>(null)
  const search = useRef<HTMLInputElement>(null)
  const list = useId()
  const options = new FontOptions(fonts)
  const current = look.font

  useEffect(() => {
    if (!open) return
    const previous = document.activeElement
    search.current?.focus()
    void catalogFamilies().then(setFamilies)
    const dismiss = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false)
    }
    const away = () => setOpen(false)
    window.addEventListener('pointerdown', dismiss)
    window.addEventListener('resize', away)
    return () => {
      window.removeEventListener('pointerdown', dismiss)
      window.removeEventListener('resize', away)
      if (previous instanceof HTMLElement) previous.focus({ preventScroll: true })
    }
  }, [open])

  const show = () => {
    const bounds = root.current?.getBoundingClientRect()
    if (bounds) setPlace({ top: bounds.bottom + 4, left: bounds.left, width: Math.max(bounds.width, 320) })
    setOpen(true)
  }

  const close = () => {
    setOpen(false)
    setQuery('')
    setActive(0)
  }

  const pick = (choice: FontChoice) => {
    close()
    onChoose(choice)
  }

  const pickCatalog = async (family: CatalogFamily) => {
    close()
    setLoading(family.family)
    try {
      const choice = await catalogFont(family, weightOf(current, look, fonts), look.italic)
      if (choice) onChoose(choice)
    } finally {
      setLoading(null)
    }
  }

  const wanted = query.trim().toLowerCase()
  const matches = (label: string) => label.toLowerCase().includes(wanted)
  const catalogSection = (title: string, source: CatalogFamily['source']): Section => {
    const found = (families ?? []).filter((family) => family.source === source && matches(family.family))
    return {
      title,
      hidden: Math.max(0, found.length - SHOWN_PER_SOURCE),
      entries: found.slice(0, SHOWN_PER_SOURCE).map((family) => ({
        key: `${source}:${family.id}`,
        label: family.family,
        detail: CATEGORY_LABELS[family.category] ?? '',
        chosen: current?.kind === 'catalog' && current.source === source && current.id === family.id,
        choose: () => void pickCatalog(family),
      })),
    }
  }
  const sections: Section[] = [
    {
      title: 'In this document',
      hidden: 0,
      entries: options.options
        .filter((option) => matches(option.label))
        .map((option) => ({
          key: `document:${option.name}`,
          label: option.label,
          detail: 'Embedded',
          chosen: current?.kind === 'document' && options.shownAs(current.name) === option.name,
          choose: () => pick({ kind: 'document', name: option.name }),
        })),
    },
    {
      title: 'Built in',
      hidden: 0,
      entries: BUNDLED.filter(({ label }) => matches(label)).map(({ family, label }) => ({
        key: `bundled:${family}`,
        label,
        detail: '',
        chosen: current?.kind === 'bundled' && current.family === family,
        choose: () => pick({ kind: 'bundled', family }),
      })),
    },
    catalogSection('Fontshare', 'fontshare'),
    catalogSection('Google Fonts & open fonts', 'fontsource'),
  ].filter((section) => section.entries.length > 0)
  const entries = sections.flatMap((section) => section.entries)

  const shown = loading !== null ? `Loading ${loading}…` : labelOf(current, options)

  function press(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      setActive(Math.min(entries.length - 1, Math.max(0, active + (event.key === 'ArrowDown' ? 1 : -1))))
    } else if (event.key === 'Enter') {
      event.preventDefault()
      entries[active]?.choose()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      close()
    }
  }

  return (
    <div ref={root} className="w-full">
      <button type="button" className={`${FIELD} ${CHEVRON} min-h-8 w-full truncate py-1 pl-2.5 text-left`} aria-haspopup="listbox" aria-expanded={open} disabled={loading !== null} onMouseDown={(event) => event.preventDefault()} onClick={() => (open ? close() : show())}>
        {shown}
      </button>
      {open && (
        <div className="fixed z-45 flex max-h-[min(420px,calc(100vh-120px))] flex-col gap-1 rounded border border-menu-border bg-surface p-2 shadow-menu" style={place ?? undefined}>
          <input
            ref={search}
            className={`${FIELD} min-h-8 w-full flex-none px-2.5 py-1`}
            type="search"
            placeholder="Search fonts"
            aria-label="Search fonts"
            role="combobox"
            aria-expanded="true"
            aria-controls={list}
            aria-activedescendant={entries[active] ? `${list}-${active}` : undefined}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value)
              setActive(0)
            }}
            onKeyDown={press}
          />
          <div id={list} className="overflow-y-auto" role="listbox" aria-label="Fonts">
            {families === null && <Hint className="px-2 py-1.5">Loading the font list…</Hint>}
            {sections.map((section) => (
              <div key={section.title} role="group" aria-label={section.title}>
                <p className="px-2 pt-2.5 pb-1 text-[11px] leading-[14px] tracking-[0.06em] text-ink uppercase">{section.title}</p>
                {section.entries.map((entry) => {
                  const index = entries.indexOf(entry)
                  return (
                    <div
                      key={entry.key}
                      id={`${list}-${index}`}
                      role="option"
                      aria-selected={entry.chosen}
                      className={`flex min-h-8 cursor-pointer items-baseline gap-2 rounded px-2 py-1.5 text-ink-strong ${index === active ? 'bg-hover' : ''} ${entry.chosen ? 'shadow-[inset_0_0_0_2px_var(--color-selection)]' : ''}`}
                      onMouseDown={(event) => event.preventDefault()}
                      onMouseEnter={() => setActive(index)}
                      onClick={entry.choose}
                    >
                      <span className="flex-1 truncate">{entry.label}</span>
                      {entry.detail && <span className="flex-none text-xs text-ink-muted">{entry.detail}</span>}
                    </div>
                  )
                })}
                {section.hidden > 0 && <Hint className="px-2 py-1.5">{section.hidden} more, keep typing to narrow them down</Hint>}
              </div>
            ))}
            {families !== null && entries.length === 0 && <Hint className="px-2 py-1.5">No font matches “{query}”.</Hint>}
          </div>
        </div>
      )}
    </div>
  )
}
