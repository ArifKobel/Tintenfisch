import type { EngineClient } from '@/services/EngineClient.ts'
import type { Priority } from '@/types/protocol.ts'
import type { CachedTile, TileCache } from '@/lib/TileCache.ts'
import type { TileRect } from '@/lib/TileGrid.ts'

export interface TileAddress {
  session: number
  pageId: string
  revision: number
  scale: number
  tile: TileRect
}

export class TileLoader {
  readonly #client: EngineClient
  readonly #cache: TileCache

  constructor(client: EngineClient, cache: TileCache) {
    this.#client = client
    this.#cache = cache
  }

  cached(address: TileAddress): CachedTile | undefined {
    return this.#cache.get(this.#key(address))
  }

  async load(address: TileAddress, priority: Priority, signal: AbortSignal): Promise<CachedTile> {
    const key = this.#key(address)
    const cached = this.#cache.get(key)
    if (cached) return cached
    const rendered = await this.#client.request(
      { kind: 'renderTile', pageId: address.pageId, scale: address.scale, ...address.tile },
      { priority, signal },
    )
    const tile = { width: rendered.width, height: rendered.height, pixels: new Uint8ClampedArray(rendered.rgba.buffer as ArrayBuffer) }
    this.#cache.set(key, tile)
    return tile
  }

  #key({ session, pageId, revision, scale, tile }: TileAddress): string {
    return `${session}:${pageId}@${revision}:${scale.toFixed(4)}:${tile.x},${tile.y},${tile.width},${tile.height}`
  }
}
