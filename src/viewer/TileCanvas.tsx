import { useRef } from 'react'
import type { TileAddress } from '@viewer/TileLoader.ts'
import { useTile } from '@viewer/useTile.ts'

export function TileCanvas({ address, pixelRatio }: { address: TileAddress; pixelRatio: number }) {
  const canvas = useRef<HTMLCanvasElement>(null)
  useTile(canvas, address, 'visible')
  const { tile } = address
  return (
    <canvas
      ref={canvas}
      className="page-tile"
      style={{
        left: tile.x / pixelRatio,
        top: tile.y / pixelRatio,
        width: tile.width / pixelRatio,
        height: tile.height / pixelRatio,
      }}
    />
  )
}
