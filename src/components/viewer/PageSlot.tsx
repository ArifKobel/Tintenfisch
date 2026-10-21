import { AnnotationLayer } from '@/components/comments/AnnotationLayer.tsx'
import type { PageSummary } from '@/types/protocol.ts'
import { FormLayer } from '@/components/forms/FormLayer.tsx'
import { ImageLayer } from '@/components/images/ImageLayer.tsx'
import { LinkLayer } from '@/components/links/LinkLayer.tsx'
import { TextLayer } from '@/components/text/TextLayer.tsx'
import { SignatureLayer } from '@/components/signing/SignatureLayer.tsx'
import { PageBackdrop } from '@/components/viewer/PageBackdrop.tsx'
import type { PagePlacement } from '@/lib/PageLayout.ts'
import { TileCanvas } from '@/components/viewer/TileCanvas.tsx'
import { TileGrid, type PixelArea } from '@/lib/TileGrid.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'

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
  const revision = useDocumentStore((state) => state.pageRevisions[page.id] ?? 0)
  const grid = new TileGrid(placement.width * pixelRatio, placement.height * pixelRatio)
  const tiles = grid.covering(visibleArea)
  return (
    <div className="absolute overflow-hidden bg-white shadow-[0_0_0_1px_rgb(0_0_0/0.04),0_1px_3px_rgb(0_0_0/0.08)]" style={{ top: placement.top, left, width: placement.width, height: placement.height }}>
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
