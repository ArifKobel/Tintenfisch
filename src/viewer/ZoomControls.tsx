import { useApplication } from '@app/ApplicationContext.ts'
import { useDocumentState } from '@document/useDocumentState.ts'
import { CSS_PIXELS_PER_POINT } from '@viewer/PageLayout.ts'
import { useViewportState } from '@viewer/useViewportState.ts'

export function ZoomControls() {
  const { viewport } = useApplication()
  const { zoom } = useViewportState()
  const document = useDocumentState()
  if (document.phase !== 'ready') return null
  const widest = Math.max(...document.pages.map((page) => page.width)) * CSS_PIXELS_PER_POINT
  return (
    <div className="zoom-controls" role="group" aria-label="Zoom">
      <button type="button" className="button" onClick={() => viewport.zoomOut()} aria-label="Zoom out">
        −
      </button>
      <span className="zoom-level">{Math.round(zoom * 100)}%</span>
      <button type="button" className="button" onClick={() => viewport.zoomIn()} aria-label="Zoom in">
        +
      </button>
      <button type="button" className="button" onClick={() => viewport.fitWidth(widest)}>
        Fit width
      </button>
    </div>
  )
}
