import { useApplication } from '@app/ApplicationContext.ts'
import type { ReadyDocument } from '@document/DocumentStore.ts'
import { IconButton } from '@ui/IconButton.tsx'
import { CSS_PIXELS_PER_POINT } from '@viewer/PageLayout.ts'
import { useViewportState } from '@viewer/useViewportState.ts'

export function ZoomControls({ document }: { document: ReadyDocument }) {
  const { viewport } = useApplication()
  const { zoom } = useViewportState()
  const widest = Math.max(...document.pages.map((page) => page.width)) * CSS_PIXELS_PER_POINT
  return (
    <div className="zoom-controls" role="group" aria-label="Zoom">
      <IconButton icon="maximize" label="Fit width" tooltip="left" onClick={() => viewport.fitWidth(widest)} />
      <IconButton icon="zoom-in" label="Zoom in" tooltip="left" onClick={() => viewport.zoomIn()} />
      <span className="zoom-level" aria-live="polite">
        {Math.round(zoom * 100)}%
      </span>
      <IconButton icon="zoom-out" label="Zoom out" tooltip="left" onClick={() => viewport.zoomOut()} />
    </div>
  )
}
