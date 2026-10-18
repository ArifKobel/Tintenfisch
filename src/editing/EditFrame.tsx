import type { PointerEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { EditSession } from '@editing/EditSession.ts'
import type { Highlight } from '@engine/protocol.ts'

const MIN_WIDTH = 12

type Drag = (session: EditSession, dx: number, dy: number) => EditSession

export function EditFrame({ frame, cssScale }: { frame: Highlight; cssScale: number }) {
  const { editing } = useApplication()

  function drag(apply: Drag) {
    return (event: PointerEvent<HTMLDivElement>) => {
      if (event.button !== 0 || !editing.session) return
      event.preventDefault()
      event.stopPropagation()
      const handle = event.currentTarget
      handle.setPointerCapture(event.pointerId)
      const start = editing.session
      const origin = { x: event.clientX, y: event.clientY }
      const move = (moved: globalThis.PointerEvent) => {
        const dx = (moved.clientX - origin.x) / cssScale
        const dy = (moved.clientY - origin.y) / cssScale
        editing.edit(() => apply(start, dx, dy))
      }
      const release = () => {
        handle.removeEventListener('pointermove', move)
        handle.removeEventListener('pointerup', release)
      }
      handle.addEventListener('pointermove', move)
      handle.addEventListener('pointerup', release)
    }
  }

  const moveBox: Drag = (session, dx, dy) => session.movedTo(session.box.dx + dx, session.box.dy + dy)
  const resizeBox: Drag = (session, dx, dy) => {
    const along = dx * Math.cos(frame.angle) + dy * Math.sin(frame.angle)
    return session.resizedTo(Math.max(MIN_WIDTH, (session.box.width ?? frame.width) + along))
  }

  return (
    <div
      className="edit-frame"
      style={{
        left: frame.x * cssScale,
        top: frame.y * cssScale,
        width: frame.width * cssScale,
        height: frame.height * cssScale,
        transform: frame.angle === 0 ? undefined : `rotate(${frame.angle}rad)`,
      }}
    >
      <div className="edit-frame-move" aria-label="Move paragraph" onPointerDown={drag(moveBox)} onMouseDown={stopFocusChange} />
      <div className="edit-frame-resize" aria-label="Resize paragraph" onPointerDown={drag(resizeBox)} onMouseDown={stopFocusChange} />
    </div>
  )
}

function stopFocusChange(event: { preventDefault(): void; stopPropagation(): void }) {
  event.preventDefault()
  event.stopPropagation()
}
