import { useState, type PointerEvent } from 'react'
import type { Box, NewFieldKind } from '@/types/protocol.ts'
import { FieldBox } from '@/components/forms/FieldBox.tsx'
import { useFormsStore } from '@/stores/formsStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

const SMALLEST = 4
const DEFAULT_SIZES: Record<NewFieldKind, [number, number]> = {
  text: [150, 20],
  multiline: [200, 60],
  checkbox: [14, 14],
  radio: [14, 14],
  combo: [120, 20],
  list: [120, 60],
}

export function FieldDesigner({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const forms = useFormsStore()
  const { fields, selected } = forms
  const { fieldKind } = useWorkspaceStore()
  const [drawn, setDrawn] = useState<{ from: [number, number]; to: [number, number] } | null>(null)

  function point(event: PointerEvent<HTMLDivElement>): [number, number] {
    const bounds = event.currentTarget.getBoundingClientRect()
    return [(event.clientX - bounds.left) / cssScale, (event.clientY - bounds.top) / cssScale]
  }

  function press(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return
    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    const at = point(event)
    setDrawn({ from: at, to: at })
    forms.select(null)
  }

  function release() {
    if (!drawn) return
    const [[x0, y0], [x1, y1]] = [drawn.from, drawn.to]
    const [width, height] = [Math.abs(x1 - x0), Math.abs(y1 - y0)]
    const [defaultWidth, defaultHeight] = DEFAULT_SIZES[fieldKind]
    const rect: Box =
      width < SMALLEST || height < SMALLEST
        ? { x: x0, y: y0, width: defaultWidth, height: defaultHeight }
        : { x: Math.min(x0, x1), y: Math.min(y0, y1), width, height }
    void forms.addField(pageId, fieldKind, rect)
    setDrawn(null)
  }

  const box = drawn && {
    left: Math.min(drawn.from[0], drawn.to[0]) * cssScale,
    top: Math.min(drawn.from[1], drawn.to[1]) * cssScale,
    width: Math.abs(drawn.to[0] - drawn.from[0]) * cssScale,
    height: Math.abs(drawn.to[1] - drawn.from[1]) * cssScale,
  }
  return (
    <div className="absolute inset-0 cursor-crosshair" onPointerDown={press} onPointerMove={(event) => drawn && setDrawn({ ...drawn, to: point(event) })} onPointerUp={release}>
      {fields.flatMap((field) =>
        field.widgets
          .filter((widget) => widget.pageId === pageId)
          .map((widget) => <FieldBox key={widget.id} field={field} widget={widget} selected={field.id === selected} cssScale={cssScale} />),
      )}
      {box && <div className="pointer-events-none absolute border border-dashed border-selection bg-selection/10" style={box} />}
    </div>
  )
}
