import { describe, expect, it } from 'vitest'
import { CSS_PIXELS_PER_POINT, PageLayout } from '@viewer/PageLayout.ts'

const pages = [
  { id: '1-0', width: 600, height: 800, rotate: 0 },
  { id: '2-0', width: 800, height: 600, rotate: 90 },
  { id: '3-0', width: 600, height: 800, rotate: 0 },
]

describe('PageLayout', () => {
  it('stacks pages with gaps at the zoomed size', () => {
    const layout = new PageLayout(pages, 0.75, 10)
    const scale = 0.75 * CSS_PIXELS_PER_POINT
    expect(layout.placements.map((placement) => placement.top)).toEqual([10, 20 + 800 * scale, 30 + 1400 * scale])
    expect(layout.totalHeight).toBe(40 + 2200 * scale)
    expect(layout.widestPage).toBe(800 * scale)
  })

  it('finds the pages that intersect a range', () => {
    const layout = new PageLayout(pages, 0.75, 10)
    const second = layout.placements[1]
    expect(layout.visible(second.top + 1, second.top + 2).map((placement) => placement.id)).toEqual(['2-0'])
    expect(layout.visible(0, layout.totalHeight).length).toBe(3)
    expect(layout.visible(layout.totalHeight + 1, layout.totalHeight + 100)).toEqual([])
  })

  it('keeps a point on its page when the zoom changes', () => {
    const before = new PageLayout(pages, 1)
    const anchor = before.anchorAt(before.placements[1].top + before.placements[1].height / 4)
    const after = new PageLayout(pages, 2)
    expect(anchor.index).toBe(1)
    expect(after.offsetOf(anchor)).toBeCloseTo(after.placements[1].top + after.placements[1].height / 4)
  })
})
