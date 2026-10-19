import { describe, expect, it } from 'vitest'
import { FontLabels } from '@editing/FontLabels.ts'
import type { PageFont } from '@engine/protocol.ts'

const font = (name: string, family: string | null): PageFont => ({ name, font: family, baseFont: name, embedded: true, family: 'sans', bold: false, italic: false, weight: null })

describe('FontLabels', () => {
  it('names fonts after their family and numbers those that share a name', () => {
    const labels = FontLabels.of([
      font('F1', 'Cabinet Grotesk Extrabold'),
      font('F3', null),
      font('F4', null),
      font('F5', 'Arial'),
      font('F6', 'Arial'),
    ])
    expect([...labels.values()]).toEqual(['Cabinet Grotesk Extrabold', 'Unnamed font 1', 'Unnamed font 2', 'Arial 1', 'Arial 2'])
  })
})
