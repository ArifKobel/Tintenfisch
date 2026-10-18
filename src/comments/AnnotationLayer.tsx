import { useState, type PointerEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { useCommentsState } from '@comments/useCommentsState.ts'
import type { MarkupRequest, Point } from '@engine/protocol.ts'
import { DRAWING_TOOLS, type Tool } from '@tools/ToolStore.ts'
import { useToolState } from '@tools/useToolState.ts'

const NOTE_HALF = 10
const SMALLEST_SHAPE = 3
const TEXT_BOX_SIZE: Point = [160, 40]

interface Sketch {
  tool: Tool
  points: Point[]
}

function finished({ tool, points }: Sketch): MarkupRequest | null {
  const [from, to] = [points[0], points[points.length - 1]]
  const size = Math.hypot(to[0] - from[0], to[1] - from[1])
  switch (tool) {
    case 'ink':
      return { kind: 'ink', strokes: [points] }
    case 'rectangle':
    case 'ellipse':
    case 'arrow':
      return size < SMALLEST_SHAPE ? null : { kind: tool, from, to }
    case 'redactArea':
      return size < SMALLEST_SHAPE ? null : { kind: 'redactArea', from, to }
    case 'textBox':
      return { kind: 'textBox', from, to: size < SMALLEST_SHAPE ? [from[0] + TEXT_BOX_SIZE[0], from[1] + TEXT_BOX_SIZE[1]] : to, text: '' }
    default:
      return null
  }
}

export function AnnotationLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { comments } = useApplication()
  const { tool, stampLabel } = useToolState()
  const { comments: all, selected } = useCommentsState()
  const [sketch, setSketch] = useState<Sketch | null>(null)
  const drawing = DRAWING_TOOLS.includes(tool)

  function point(event: PointerEvent<HTMLDivElement>): Point {
    const bounds = event.currentTarget.getBoundingClientRect()
    return [(event.clientX - bounds.left) / cssScale, (event.clientY - bounds.top) / cssScale]
  }

  function press(event: PointerEvent<HTMLDivElement>) {
    if (!drawing || event.button !== 0) return
    event.preventDefault()
    const at = point(event)
    if (tool === 'note') void comments.add(pageId, { kind: 'note', at: [at[0] - NOTE_HALF, at[1] - NOTE_HALF], text: '' })
    else if (tool === 'stamp') void comments.add(pageId, { kind: 'stamp', at, label: stampLabel })
    else {
      event.currentTarget.setPointerCapture(event.pointerId)
      setSketch({ tool, points: [at] })
    }
  }

  function move(event: PointerEvent<HTMLDivElement>) {
    if (!sketch) return
    const at = point(event)
    setSketch({ ...sketch, points: sketch.tool === 'ink' ? [...sketch.points, at] : [sketch.points[0], at] })
  }

  function release() {
    if (!sketch) return
    const markup = finished(sketch)
    if (markup) void comments.add(pageId, markup)
    setSketch(null)
  }

  const onPage = tool === 'select' ? all.filter((comment) => comment.pageId === pageId) : []
  return (
    <div
      className={`annotation-layer${drawing ? ' annotation-layer-drawing' : ''}`}
      onPointerDown={press}
      onPointerMove={move}
      onPointerUp={release}
    >
      {onPage.map((comment) => (
        <button
          key={comment.id}
          type="button"
          className={`comment-box${comment.id === selected ? ' comment-box-selected' : ''}`}
          aria-label={`${comment.subtype} comment`}
          style={{
            left: comment.rect.x * cssScale,
            top: comment.rect.y * cssScale,
            width: comment.rect.width * cssScale,
            height: comment.rect.height * cssScale,
          }}
          onMouseDown={(event) => event.stopPropagation()}
          onClick={() => comments.select(comment.id)}
        />
      ))}
      {sketch && <SketchPreview sketch={sketch} cssScale={cssScale} />}
    </div>
  )
}

function SketchPreview({ sketch, cssScale }: { sketch: Sketch; cssScale: number }) {
  const points = sketch.points.map(([x, y]) => [x * cssScale, y * cssScale])
  const [[x0, y0], [x1, y1]] = [points[0], points[points.length - 1]]
  const box = { x: Math.min(x0, x1), y: Math.min(y0, y1), width: Math.abs(x1 - x0), height: Math.abs(y1 - y0) }
  return (
    <svg className="annotation-sketch">
      {sketch.tool === 'ink' && <polyline points={points.map((point) => point.join(',')).join(' ')} />}
      {(sketch.tool === 'rectangle' || sketch.tool === 'redactArea' || sketch.tool === 'textBox') && <rect {...box} />}
      {sketch.tool === 'ellipse' && <ellipse cx={box.x + box.width / 2} cy={box.y + box.height / 2} rx={box.width / 2} ry={box.height / 2} />}
      {sketch.tool === 'arrow' && <line x1={x0} y1={y0} x2={x1} y2={y1} />}
    </svg>
  )
}
