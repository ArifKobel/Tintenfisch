import { AnnotationLayer } from '@comments/AnnotationLayer.tsx'
import type { PageSummary } from '@engine/protocol.ts'
import { FormLayer } from '@forms/FormLayer.tsx'
import { ImageLayer } from '@images/ImageLayer.tsx'
import { LinkLayer } from '@navigation/LinkLayer.tsx'
import { TextLayer } from '@selection/TextLayer.tsx'
import { SignatureLayer } from '@signing/SignatureLayer.tsx'
import { PageBackdrop } from '@viewer/PageBackdrop.tsx'
import type { PagePlacement } from '@viewer/PageLayout.ts'
import { TileCanvas } from '@viewer/TileCanvas.tsx'
import { TileGrid, type PixelArea } from '@viewer/TileGrid.ts'
import { usePageRevision } from '@viewer/usePageRevision.ts'

export interface PageSlotProps {
  session: number
  page: PageSummary
  placement: PagePlacement
  left: number
  scale: number
  cssScale: number
  pixelRatio: number
  visibleArea: PixelArea
}

export function PageSlot({ session, page, placement, left, scale, cssScale, pixelRatio, visibleArea }: PageSlotProps) {
  const revision = usePageRevision(page.id)
  const grid = new TileGrid(placement.width * pixelRatio, placement.height * pixelRatio)
  const tiles = grid.covering(visibleArea)
  return (
    <div className="page-slot" style={{ top: placement.top, left, width: placement.width, height: placement.height }}>
      <PageBackdrop session={session} page={page} />
      {tiles.map((tile) => (
        <TileCanvas
          key={`${scale}:${tile.x},${tile.y}`}
          address={{ session, pageId: page.id, revision, scale, tile }}
          pixelRatio={pixelRatio}
        />
      ))}
      <TextLayer pageId={page.id} cssScale={cssScale} />
      <ImageLayer pageId={page.id} cssScale={cssScale} />
      <LinkLayer pageId={page.id} cssScale={cssScale} />
      <FormLayer pageId={page.id} cssScale={cssScale} />
      <AnnotationLayer pageId={page.id} cssScale={cssScale} />
      <SignatureLayer pageId={page.id} cssScale={cssScale} />
    </div>
  )
}
