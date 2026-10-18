import { useRef } from 'react'
import type { PageSummary } from '@engine/protocol.ts'
import { usePageRevision } from '@viewer/usePageRevision.ts'
import { useTile } from '@viewer/useTile.ts'

const BACKDROP_WIDTH = 480

export function PageBackdrop({ session, page }: { session: number; page: PageSummary }) {
  const canvas = useRef<HTMLCanvasElement>(null)
  const revision = usePageRevision(page.id)
  const scale = BACKDROP_WIDTH / page.width
  const tile = { x: 0, y: 0, width: Math.ceil(page.width * scale), height: Math.ceil(page.height * scale) }
  useTile(canvas, { session, pageId: page.id, revision, scale, tile }, 'visible')
  return <canvas ref={canvas} className="page-backdrop" />
}
