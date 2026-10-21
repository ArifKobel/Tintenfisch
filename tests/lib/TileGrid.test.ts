import { describe, expect, it } from 'vitest'
import { TileGrid } from '@/lib/TileGrid.ts'

describe('TileGrid', () => {
  it('covers the visible area with tiles clipped to the page', () => {
    const grid = new TileGrid(1000, 700, 512)
    expect(grid.covering({ left: 0, top: 0, right: 1000, bottom: 700 })).toEqual([
      { x: 0, y: 0, width: 512, height: 512 },
      { x: 512, y: 0, width: 488, height: 512 },
      { x: 0, y: 512, width: 512, height: 188 },
      { x: 512, y: 512, width: 488, height: 188 },
    ])
  })

  it('returns only the tiles touching a partly visible area', () => {
    const grid = new TileGrid(2048, 2048, 512)
    const tiles = grid.covering({ left: 600, top: -300, right: 700, bottom: 10 })
    expect(tiles).toEqual([{ x: 512, y: 0, width: 512, height: 512 }])
    expect(grid.covering({ left: 3000, top: 0, right: 4000, bottom: 100 })).toEqual([])
  })
})
