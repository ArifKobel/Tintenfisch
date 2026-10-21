import { describe, expect, it } from 'vitest'
import { FontOptions } from '@/lib/FontOptions.ts'
import type { PageFont } from '@/types/protocol.ts'

const font = (name: string, family: string | null): PageFont => ({ name, font: family, baseFont: name, embedded: true, family: 'sans', bold: false, italic: false, weight: null })

describe('FontOptions', () => {
  it('offers each named font once and numbers fonts without a name', () => {
    const options = new FontOptions([
      font('F1', 'Cabinet Grotesk Extrabold'),
      font('F3', null),
      font('F4', null),
      font('TfFallback0', 'Cabinet Grotesk Extrabold'),
      font('TfFallback1', 'Cabinet Grotesk Extrabold'),
    ])
    expect(options.options).toEqual([
      { name: 'F1', label: 'Cabinet Grotesk Extrabold' },
      { name: 'F3', label: 'Unnamed font 1' },
      { name: 'F4', label: 'Unnamed font 2' },
    ])
    expect(options.shownAs('TfFallback1')).toBe('F1')
    expect(options.shownAs('F4')).toBe('F4')
  })
})
