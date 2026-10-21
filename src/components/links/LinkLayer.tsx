import { useEffect, useState, type PointerEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import { useDocumentState } from '@/hooks/useDocumentState.ts'
import type { Box, Target } from '@/types/protocol.ts'
import { LinkBox } from '@/components/links/LinkBox.tsx'
import { useLinksState } from '@/hooks/useLinksState.ts'
import { useToolState } from '@/hooks/useToolState.ts'
import { usePageRevision } from '@/hooks/usePageRevision.ts'

const SMALLEST_DRAWN = 4

export function LinkLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { links } = useApplication()
  const { tool } = useToolState()
  const { pages, selected } = useLinksState()
  const document = useDocumentState()
  const revision = usePageRevision(pageId)
  const [drawn, setDrawn] = useState<Box | null>(null)
  const active = tool === 'select' || tool === 'link'

  useEffect(() => {
    if (active) void links.load(pageId)
  }, [links, pageId, revision, active])

  if (!active) return null
  const pageLinks = pages[pageId] ?? []
  const pageNumber = (id: string) => (document.phase === 'ready' ? document.pages.findIndex((page) => page.id === id) + 1 : 0)
  const describe = (target: Target | null) => (target?.kind === 'uri' ? target.uri : target ? `Go to page ${pageNumber(target.pageId)}` : 'Link')

  if (tool === 'select') {
    return (
      <div className="link-layer">
        {pageLinks.map((link) => (
          <button
            key={link.id}
            type="button"
            className="link-spot"
            title={describe(link.target)}
            aria-label={describe(link.target)}
            style={{ left: link.rect.x * cssScale, top: link.rect.y * cssScale, width: link.rect.width * cssScale, height: link.rect.height * cssScale }}
            onClick={() => links.follow(link)}
          />
        ))}
      </div>
    )
  }

  function start(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || event.target !== event.currentTarget) return
    links.select(null)
    const layer = event.currentTarget
    layer.setPointerCapture(event.pointerId)
    const bounds = layer.getBoundingClientRect()
    const point = (moving: { clientX: number; clientY: number }) => [(moving.clientX - bounds.left) / cssScale, (moving.clientY - bounds.top) / cssScale]
    const [x0, y0] = point(event)
    let rect: Box = { x: x0, y: y0, width: 0, height: 0 }
    const follow = (moving: globalThis.PointerEvent) => {
      const [x1, y1] = point(moving)
      rect = { x: Math.min(x0, x1), y: Math.min(y0, y1), width: Math.abs(x1 - x0), height: Math.abs(y1 - y0) }
      setDrawn(rect)
    }
    const release = () => {
      layer.removeEventListener('pointermove', follow)
      layer.removeEventListener('pointerup', release)
      setDrawn(null)
      if (rect.width >= SMALLEST_DRAWN && rect.height >= SMALLEST_DRAWN) links.draw(pageId, rect)
    }
    layer.addEventListener('pointermove', follow)
    layer.addEventListener('pointerup', release)
  }

  return (
    <div className="link-layer link-layer-editing" onPointerDown={start}>
      {pageLinks.map((link) => (
        <LinkBox key={link.id} link={link} pageId={pageId} selected={selected?.id === link.id} cssScale={cssScale} />
      ))}
      {drawn && (
        <div className="link-box link-box-drawn" style={{ left: drawn.x * cssScale, top: drawn.y * cssScale, width: drawn.width * cssScale, height: drawn.height * cssScale }} />
      )}
    </div>
  )
}
