import type { PointerEvent } from 'react'
import type { Highlight } from '@/types/protocol.ts'
import { useImagesStore, type ImageChoice } from '@/stores/imagesStore.ts'

const SMALLEST = 8

type Reshape = (start: Highlight, dx: number, dy: number) => Highlight

const moved: Reshape = (start, dx, dy) => ({ ...start, x: start.x + dx, y: start.y + dy })

const resized: Reshape = (start, dx, dy) => {
  const along = dx * Math.cos(start.angle) + dy * Math.sin(start.angle)
  const down = -dx * Math.sin(start.angle) + dy * Math.cos(start.angle)
  const factor = Math.max((start.width + along) / start.width, (start.height + down) / start.height, SMALLEST / Math.min(start.width, start.height))
  return { ...start, width: start.width * factor, height: start.height * factor }
}

function placement(frame: Highlight, cssScale: number) {
  return {
    left: `${frame.x * cssScale}px`,
    top: `${frame.y * cssScale}px`,
    width: `${frame.width * cssScale}px`,
    height: `${frame.height * cssScale}px`,
    transform: frame.angle === 0 ? '' : `rotate(${frame.angle}rad)`,
  }
}

export function ImageBox({ frame, choice, selected, cssScale }: { frame: Highlight; choice: ImageChoice; selected: boolean; cssScale: number }) {
  const images = useImagesStore()

  function drag(reshape: Reshape) {
    return (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return
      event.preventDefault()
      event.stopPropagation()
      const handle = event.currentTarget
      const box = handle.closest<HTMLElement>('.image-box')
      handle.setPointerCapture(event.pointerId)
      const origin = { x: event.clientX, y: event.clientY }
      let last = frame
      const follow = (moving: globalThis.PointerEvent) => {
        last = reshape(frame, (moving.clientX - origin.x) / cssScale, (moving.clientY - origin.y) / cssScale)
        if (box) Object.assign(box.style, placement(last, cssScale))
      }
      const release = () => {
        handle.removeEventListener('pointermove', follow)
        handle.removeEventListener('pointerup', release)
        if (last !== frame) void images.move(choice, last)
      }
      handle.addEventListener('pointermove', follow)
      handle.addEventListener('pointerup', release)
    }
  }

  return (
    <div
      className={`image-box${selected ? ' image-box-selected' : ''}`}
      style={placement(frame, cssScale)}
      onPointerDown={selected ? drag(moved) : undefined}
      onMouseDown={selected ? (event) => event.stopPropagation() : undefined}
      aria-label={selected ? 'Selected image' : undefined}
    >
      {selected && <span className="image-box-resize" aria-label="Resize image" onPointerDown={drag(resized)} />}
    </div>
  )
}
