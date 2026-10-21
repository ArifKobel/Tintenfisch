import { useEffect, type RefObject } from 'react'
import type { CachedTile } from '@/lib/TileCache.ts'
import { CancelledError } from '@/services/EngineClient.ts'
import { cachedTile, loadTile, type TileAddress } from '@/services/tiles.ts'
import type { Priority } from '@/types/protocol.ts'

function paint(canvas: HTMLCanvasElement, tile: CachedTile) {
  canvas.width = tile.width
  canvas.height = tile.height
  canvas.getContext('2d')?.putImageData(new ImageData(tile.pixels, tile.width, tile.height), 0, 0)
}

export function useTile(canvas: RefObject<HTMLCanvasElement | null>, address: TileAddress | null, priority: Priority) {
  const { session, pageId, revision, scale } = address ?? { session: 0, pageId: '', revision: 0, scale: 0 }
  const { x, y, width, height } = address?.tile ?? { x: 0, y: 0, width: 0, height: 0 }
  const active = address !== null
  useEffect(() => {
    if (!active || !canvas.current) return
    const target = canvas.current
    const located = { session, pageId, revision, scale, tile: { x, y, width, height } }
    const cached = cachedTile(located)
    if (cached) {
      paint(target, cached)
      return
    }
    const controller = new AbortController()
    loadTile(located, priority, controller.signal)
      .then((tile) => paint(target, tile))
      .catch((error: unknown) => {
        if (!(error instanceof CancelledError)) throw error
      })
    return () => controller.abort()
  }, [canvas, active, session, pageId, revision, scale, x, y, width, height, priority])
}
