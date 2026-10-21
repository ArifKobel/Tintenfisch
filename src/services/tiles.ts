import { engine } from '@/services/EngineClient.ts'
import { TileCache, type CachedTile } from '@/lib/TileCache.ts'
import type { TileRect } from '@/lib/TileGrid.ts'
import type { Priority } from '@/types/protocol.ts'

export interface TileAddress {
  session: number
  pageId: string
  revision: number
  scale: number
  tile: TileRect
}

const cache = new TileCache()

function keyOf({ session, pageId, revision, scale, tile }: TileAddress): string {
  return `${session}:${pageId}@${revision}:${scale.toFixed(4)}:${tile.x},${tile.y},${tile.width},${tile.height}`
}

export function cachedTile(address: TileAddress): CachedTile | undefined {
  return cache.get(keyOf(address))
}

export async function loadTile(address: TileAddress, priority: Priority, signal: AbortSignal): Promise<CachedTile> {
  const key = keyOf(address)
  const cached = cache.get(key)
  if (cached) return cached
  const rendered = await engine.request({ kind: 'renderTile', pageId: address.pageId, scale: address.scale, ...address.tile }, { priority, signal })
  const tile = { width: rendered.width, height: rendered.height, pixels: new Uint8ClampedArray(rendered.rgba.buffer as ArrayBuffer) }
  cache.set(key, tile)
  return tile
}
