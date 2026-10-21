import type { PointerEvent } from 'react'
import type { Box, FormField, FormWidget } from '@/types/protocol.ts'
import { useFormsStore } from '@/stores/formsStore.ts'
import { RESIZE_HANDLE } from '@/components/ui/controlStyles.ts'

const SMALLEST = 6

type Reshape = (start: Box, dx: number, dy: number) => Box

const moved: Reshape = (start, dx, dy) => ({ ...start, x: start.x + dx, y: start.y + dy })
const resized: Reshape = (start, dx, dy) => ({ ...start, width: Math.max(SMALLEST, start.width + dx), height: Math.max(SMALLEST, start.height + dy) })

export function FieldBox({ field, widget, selected, cssScale }: { field: FormField; widget: FormWidget; selected: boolean; cssScale: number }) {
  const forms = useFormsStore()

  function drag(reshape: Reshape) {
    return (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return
      event.preventDefault()
      event.stopPropagation()
      forms.select(field.id)
      const handle = event.currentTarget
      const box = handle.closest<HTMLElement>('.field-box')
      handle.setPointerCapture(event.pointerId)
      const origin = { x: event.clientX, y: event.clientY }
      let last = widget.rect
      const follow = (moving: globalThis.PointerEvent) => {
        last = reshape(widget.rect, (moving.clientX - origin.x) / cssScale, (moving.clientY - origin.y) / cssScale)
        if (box) Object.assign(box.style, { left: `${last.x * cssScale}px`, top: `${last.y * cssScale}px`, width: `${last.width * cssScale}px`, height: `${last.height * cssScale}px` })
      }
      const release = () => {
        handle.removeEventListener('pointermove', follow)
        handle.removeEventListener('pointerup', release)
        if (last !== widget.rect) void forms.moveWidget(widget.pageId, widget.id, last)
      }
      handle.addEventListener('pointermove', follow)
      handle.addEventListener('pointerup', release)
    }
  }

  return (
    <div
      className={`absolute cursor-move border border-selection bg-selection/10 ${selected ? 'outline-2 outline-selection/35' : 'border-dashed'}`}
      style={{ left: widget.rect.x * cssScale, top: widget.rect.y * cssScale, width: widget.rect.width * cssScale, height: widget.rect.height * cssScale }}
      onPointerDown={drag(moved)}
      role="button"
      aria-label={`Field ${field.name}`}
    >
      <span className="absolute bottom-full left-0 rounded-[2px] bg-selection px-1 text-[10px] leading-[14px] whitespace-nowrap text-white">{field.name}</span>
      {selected && <span className={RESIZE_HANDLE} aria-label="Resize field" onPointerDown={drag(resized)} />}
    </div>
  )
}
