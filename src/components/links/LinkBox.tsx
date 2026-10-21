import type { PointerEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { Box, DocumentLink } from '@/types/protocol.ts'

const SMALLEST = 6

type Reshape = (start: Box, dx: number, dy: number) => Box

const moved: Reshape = (start, dx, dy) => ({ ...start, x: start.x + dx, y: start.y + dy })
const resized: Reshape = (start, dx, dy) => ({ ...start, width: Math.max(SMALLEST, start.width + dx), height: Math.max(SMALLEST, start.height + dy) })

const placement = (rect: Box, cssScale: number) => ({
  left: `${rect.x * cssScale}px`,
  top: `${rect.y * cssScale}px`,
  width: `${rect.width * cssScale}px`,
  height: `${rect.height * cssScale}px`,
})

export function LinkBox({ link, pageId, selected, cssScale }: { link: DocumentLink; pageId: string; selected: boolean; cssScale: number }) {
  const { links } = useApplication()
  const choice = { pageId, id: link.id }

  function drag(reshape: Reshape) {
    return (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return
      event.preventDefault()
      event.stopPropagation()
      links.select(choice)
      const handle = event.currentTarget
      const box = handle.closest<HTMLElement>('.link-box')
      handle.setPointerCapture(event.pointerId)
      const origin = { x: event.clientX, y: event.clientY }
      let last = link.rect
      const follow = (moving: globalThis.PointerEvent) => {
        last = reshape(link.rect, (moving.clientX - origin.x) / cssScale, (moving.clientY - origin.y) / cssScale)
        if (box) Object.assign(box.style, placement(last, cssScale))
      }
      const release = () => {
        handle.removeEventListener('pointermove', follow)
        handle.removeEventListener('pointerup', release)
        if (last !== link.rect) void links.move(choice, last)
      }
      handle.addEventListener('pointermove', follow)
      handle.addEventListener('pointerup', release)
    }
  }

  return (
    <div
      className={`link-box${selected ? ' link-box-selected' : ''}`}
      style={placement(link.rect, cssScale)}
      role="button"
      aria-label="Link"
      onPointerDown={drag(moved)}
      onDoubleClick={() => links.editSelected()}
    >
      {selected && <span className="link-box-resize" aria-label="Resize link" onPointerDown={drag(resized)} />}
    </div>
  )
}
