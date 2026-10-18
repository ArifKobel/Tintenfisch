import { describe, expect, it } from 'vitest'
import { TileCache } from '@viewer/TileCache.ts'

const tile = (bytes: number) => ({ width: 1, height: bytes / 4, pixels: new Uint8ClampedArray(bytes) })

describe('TileCache', () => {
  it('evicts the least recently used tiles once over budget', () => {
    const cache = new TileCache(100)
    cache.set('a', tile(40))
    cache.set('b', tile(40))
    cache.get('a')
    cache.set('c', tile(40))
    expect(cache.get('b')).toBeUndefined()
    expect(cache.get('a')).toBeDefined()
    expect(cache.get('c')).toBeDefined()
    expect(cache.bytes).toBe(80)
  })

  it('keeps a single tile that alone exceeds the budget and accounts replacements', () => {
    const cache = new TileCache(10)
    cache.set('huge', tile(40))
    expect(cache.get('huge')).toBeDefined()
    cache.set('huge', tile(8))
    expect(cache.bytes).toBe(8)
  })
})
