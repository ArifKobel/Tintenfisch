import { describe, expect, it } from 'vitest'
import { FontLabels } from '@editing/FontLabels.ts'

describe('FontLabels', () => {
  it('names fonts after their family and numbers those that share a name', () => {
    const labels = FontLabels.of([
      { name: 'F1', font: 'Cabinet Grotesk Extrabold' },
      { name: 'F3', font: null },
      { name: 'F4', font: null },
      { name: 'F5', font: 'Arial' },
      { name: 'F6', font: 'Arial' },
    ])
    expect([...labels.values()]).toEqual(['Cabinet Grotesk Extrabold', 'Unnamed font 1', 'Unnamed font 2', 'Arial 1', 'Arial 2'])
  })
})
