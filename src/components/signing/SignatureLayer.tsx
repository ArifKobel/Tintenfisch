import { useState, type PointerEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { Box, Point } from '@/types/protocol.ts'
import { useSignatureState } from '@/hooks/useSignatureState.ts'
import { useToolState } from '@/hooks/useToolState.ts'

const DEFAULT_WIDTH = 180
const DEFAULT_HEIGHT = 60
const SMALLEST_DRAG = 8

function boxBetween([x0, y0]: Point, [x1, y1]: Point): Box {
  return { x: Math.min(x0, x1), y: Math.min(y0, y1), width: Math.abs(x1 - x0), height: Math.abs(y1 - y0) }
}

export function SignatureLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { signing } = useApplication()
  const { tool } = useToolState()
  const { mark } = useSignatureState()
  const [drag, setDrag] = useState<{ from: Point; to: Point } | null>(null)
  if (tool !== 'sign' || !mark) return null

  function point(event: PointerEvent<HTMLDivElement>): Point {
    const bounds = event.currentTarget.getBoundingClientRect()
    return [(event.clientX - bounds.left) / cssScale, (event.clientY - bounds.top) / cssScale]
  }

  function press(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return
    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    const at = point(event)
    setDrag({ from: at, to: at })
  }

  function release() {
    if (!drag) return
    const dragged = boxBetween(drag.from, drag.to)
    const [x, y] = drag.from
    const rect =
      dragged.width < SMALLEST_DRAG || dragged.height < SMALLEST_DRAG
        ? { x: x - DEFAULT_WIDTH / 2, y: y - DEFAULT_HEIGHT / 2, width: DEFAULT_WIDTH, height: DEFAULT_HEIGHT }
        : dragged
    setDrag(null)
    void signing.place(pageId, rect)
  }

  const shown = drag && boxBetween(drag.from, drag.to)
  return (
    <div
      className="signature-layer"
      onPointerDown={press}
      onPointerMove={(event) => drag && setDrag({ ...drag, to: point(event) })}
      onPointerUp={release}
    >
      {shown && (
        <div
          className="signature-placement"
          style={{ left: shown.x * cssScale, top: shown.y * cssScale, width: shown.width * cssScale, height: shown.height * cssScale }}
        />
      )}
    </div>
  )
}
