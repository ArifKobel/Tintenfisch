import { describe, expect, it } from 'vitest'
import { FontLabels } from '@editing/FontLabels.ts'

describe('FontLabels', () => {
  it('names fonts after their family and tells apart those that share one', () => {
    const labels = FontLabels.of([
      { name: 'F1', font: 'Cabinet Grotesk Extrabold' },
      { name: 'F3', font: 'false' },
      { name: 'F4', font: 'false' },
      { name: 'F5', font: '' },
    ])
    expect([...labels.values()]).toEqual(['Cabinet Grotesk Extrabold', 'false (F3)', 'false (F4)', 'F5'])
  })
})
