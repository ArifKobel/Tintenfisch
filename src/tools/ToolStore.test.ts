import { describe, expect, it } from 'vitest'
import { MARKUP_COLORS, ToolStore } from '@tools/ToolStore.ts'

describe('ToolStore', () => {
  it('keeps a color for highlights apart from the one every other tool draws in', () => {
    const tools = new ToolStore()
    expect(tools.getSnapshot().tool).toBe('select')
    tools.choose('ink')
    tools.setColor(MARKUP_COLORS[1])
    tools.choose('highlight')
    expect(tools.activeColor).toBe(MARKUP_COLORS[3])
    tools.setColor(MARKUP_COLORS[2])
    tools.choose('rectangle')
    expect(tools.activeColor).toBe(MARKUP_COLORS[1])
    expect(tools.getSnapshot()).toMatchObject({ color: MARKUP_COLORS[1], highlightColor: MARKUP_COLORS[2] })
  })
})
