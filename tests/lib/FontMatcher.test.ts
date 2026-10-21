import { describe, expect, it } from 'vitest'
import type { PageFont } from '@/types/protocol.ts'
import { parseCatalog } from '@/services/fontCatalog.ts'
import { FontMatcher } from '@/lib/FontMatcher.ts'

const families = parseCatalog({
  fontsource: [
    ['inter', 'Inter', 'sans-serif', [400, 700], true, true],
    ['source-serif-4', 'Source Serif 4', 'serif', [400, 700], false, false],
    ['jetbrains-mono', 'JetBrains Mono', 'monospace', [400], false, false],
    ['roboto', 'Roboto', 'sans-serif', [300, 400, 500, 700], true, true],
  ],
  fontshare: [
    [
      'cabinet-grotesk',
      'Cabinet Grotesk',
      'sans',
      [
        [400, false, 'regular'],
        [700, false, 'bold'],
        [800, false, 'extrabold'],
      ],
    ],
  ],
})

const font = (name: string | null, traits: Partial<PageFont> = {}): PageFont => ({
  name: 'F1',
  font: name,
  baseFont: 'AAAAAA+x',
  embedded: true,
  family: 'sans',
  bold: false,
  italic: false,
  weight: null,
  ...traits,
})

describe('FontMatcher', () => {
  const matcher = new FontMatcher(families)

  it('finds the family of a font by its name and the weight its style words give', () => {
    const match = matcher.match(font('Cabinet Grotesk Bold', { weight: 500 }))
    expect(match).toMatchObject({ exact: true, family: { id: 'cabinet-grotesk', source: 'fontshare' }, style: { weight: 700 } })
    expect(match?.style.urls).toEqual(['https://cdn.fontshare.com/wf/bold.ttf'])
  })

  it('falls back to the weight in the font when the name says nothing about it', () => {
    expect(matcher.match(font('Roboto', { weight: 500 }))?.style).toMatchObject({ weight: 500, italic: false })
    expect(matcher.match(font('Roboto Italic', { bold: true }))?.style).toMatchObject({ weight: 700, italic: true })
    expect(matcher.match(font('Roboto Bold'))?.style.urls).toEqual([
      'https://cdn.jsdelivr.net/fontsource/fonts/roboto@latest/latin-700-normal.ttf',
      'https://cdn.jsdelivr.net/fontsource/fonts/roboto@latest/latin-ext-700-normal.ttf',
    ])
  })

  it('takes a similar open font of the same kind when the family is unknown', () => {
    expect(matcher.match(font('Helvetica Neue', { bold: true }))).toMatchObject({ exact: false, family: { id: 'inter' }, style: { weight: 700 } })
    expect(matcher.match(font(null, { family: 'serif', weight: 400 }))).toMatchObject({ exact: false, family: { id: 'source-serif-4' } })
    expect(matcher.match(font('Consolas', { family: 'mono', bold: true }))?.style.weight).toBe(400)
  })
})
