import { describe, expect, it } from 'vitest'
import { TextBlocks } from '@/lib/TextBlocks.ts'

describe('TextBlocks', () => {
  const upright = { x: 50, y: 100, width: 200, height: 30, angle: 0 }
  const turned = { x: 300, y: 100, width: 100, height: 20, angle: Math.PI / 2 }

  it('finds points inside a block and just around its edge', () => {
    expect(TextBlocks.contain([upright], 60, 110)).toBe(true)
    expect(TextBlocks.contain([upright], 251, 131)).toBe(true)
    expect(TextBlocks.contain([upright], 260, 110)).toBe(false)
    expect(TextBlocks.contain([upright], 60, 140)).toBe(false)
  })

  it('follows the angle of turned text', () => {
    expect(TextBlocks.contain([turned], 290, 150)).toBe(true)
    expect(TextBlocks.contain([turned], 350, 110)).toBe(false)
  })
})
