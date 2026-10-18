export const TILE_SIZE = 512

export interface TileRect {
  x: number
  y: number
  width: number
  height: number
}

export interface PixelArea {
  left: number
  top: number
  right: number
  bottom: number
}

export class TileGrid {
  readonly #width: number
  readonly #height: number
  readonly #tileSize: number

  constructor(width: number, height: number, tileSize = TILE_SIZE) {
    this.#width = Math.ceil(width)
    this.#height = Math.ceil(height)
    this.#tileSize = tileSize
  }

  covering(area: PixelArea): TileRect[] {
    const size = this.#tileSize
    const left = Math.max(0, Math.floor(area.left / size))
    const top = Math.max(0, Math.floor(area.top / size))
    const right = Math.min(Math.ceil(this.#width / size), Math.ceil(area.right / size))
    const bottom = Math.min(Math.ceil(this.#height / size), Math.ceil(area.bottom / size))
    const tiles: TileRect[] = []
    for (let row = top; row < bottom; row++) {
      for (let column = left; column < right; column++) {
        const x = column * size
        const y = row * size
        tiles.push({ x, y, width: Math.min(size, this.#width - x), height: Math.min(size, this.#height - y) })
      }
    }
    return tiles
  }
}
