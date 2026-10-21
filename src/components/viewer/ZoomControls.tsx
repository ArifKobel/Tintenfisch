import { IconButton } from '@/components/ui/IconButton.tsx'
import { Tooltip } from '@/components/ui/Tooltip.tsx'
import { CSS_PIXELS_PER_POINT } from '@/lib/PageLayout.ts'
import type { ReadyDocument } from '@/stores/documentStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'

export function ZoomControls({ document }: { document: ReadyDocument }) {
  const { zoom, fitWidth, zoomIn, zoomOut, setZoom } = useViewportStore()
  const widest = Math.max(...document.pages.map((page) => page.width)) * CSS_PIXELS_PER_POINT
  return (
    <div className="mt-auto flex flex-col items-center gap-2 border-t border-divider py-2" role="group" aria-label="Zoom">
      <IconButton icon="maximize" label="Fit width" tooltip="left" onClick={() => fitWidth(widest)} />
      <IconButton icon="zoom-in" label="Zoom in" tooltip="left" onClick={zoomIn} />
      <button type="button" className="group/tip relative cursor-pointer rounded px-1 py-0.5 text-[11px] leading-[14px] text-ink-muted tabular-nums hover:text-ink-strong" aria-live="polite" aria-label={`Zoom ${Math.round(zoom * 100)}%, reset to 100%`} onClick={() => setZoom(1)}>
        {Math.round(zoom * 100)}%
        <Tooltip side="left">Reset to 100%</Tooltip>
      </button>
      <IconButton icon="zoom-out" label="Zoom out" tooltip="left" onClick={zoomOut} />
    </div>
  )
}
