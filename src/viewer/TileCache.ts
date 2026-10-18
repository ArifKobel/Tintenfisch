export interface CachedTile {
  width: number
  height: number
  pixels: Uint8ClampedArray<ArrayBuffer>
}

export const DEFAULT_TILE_BUDGET = 256 * 1024 * 1024

export class TileCache {
  readonly #budget: number
  readonly #tiles = new Map<string, CachedTile>()
  #bytes = 0

  constructor(budget = DEFAULT_TILE_BUDGET) {
    this.#budget = budget
  }

  get bytes(): number {
    return this.#bytes
  }

  get(key: string): CachedTile | undefined {
    const tile = this.#tiles.get(key)
    if (tile) {
      this.#tiles.delete(key)
      this.#tiles.set(key, tile)
    }
    return tile
  }

  set(key: string, tile: CachedTile) {
    this.delete(key)
    this.#tiles.set(key, tile)
    this.#bytes += tile.pixels.byteLength
    for (const [oldest, cached] of this.#tiles) {
      if (this.#bytes <= this.#budget || oldest === key) break
      this.#tiles.delete(oldest)
      this.#bytes -= cached.pixels.byteLength
    }
  }

  delete(key: string) {
    const existing = this.#tiles.get(key)
    if (!existing) return
    this.#tiles.delete(key)
    this.#bytes -= existing.pixels.byteLength
  }

  clear() {
    this.#tiles.clear()
    this.#bytes = 0
  }
}
