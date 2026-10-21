import { useRef } from 'react'
import { useTile } from '@/hooks/useTile.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import type { PageSummary } from '@/types/protocol.ts'

const BACKDROP_WIDTH = 480

export function PageBackdrop({ session, page }: { session: number; page: PageSummary }) {
  const canvas = useRef<HTMLCanvasElement>(null)
  const revision = useDocumentStore((state) => state.pageRevisions[page.id] ?? 0)
  const scale = BACKDROP_WIDTH / page.width
  const tile = { x: 0, y: 0, width: Math.ceil(page.width * scale), height: Math.ceil(page.height * scale) }
  useTile(canvas, { session, pageId: page.id, revision, scale, tile }, 'visible')
  return <canvas ref={canvas} className="page-backdrop" />
}
