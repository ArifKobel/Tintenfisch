import { useEffect, useRef, useState, type DragEvent, type MouseEvent } from 'react'
import type { PageSummary } from '@/types/protocol.ts'
import { useTile } from '@/hooks/useTile.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'

export const THUMBNAIL_WIDTH = 120

export interface ThumbnailGesture {
  toggle: boolean
  range: boolean
}

export interface ThumbnailProps {
  session: number
  page: PageSummary
  number: number
  current: boolean
  selected: boolean
  dropBefore: boolean
  onChoose: (gesture: ThumbnailGesture) => void
  onDragStart: (event: DragEvent) => void
  onHover: (lowerHalf: boolean) => void
}

export function Thumbnail({ session, page, number, current, selected, dropBefore, onChoose, onDragStart, onHover }: ThumbnailProps) {
  const canvas = useRef<HTMLCanvasElement>(null)
  const button = useRef<HTMLButtonElement>(null)
  const [nearView, setNearView] = useState(false)
  const revision = useDocumentStore((state) => state.pageRevisions[page.id] ?? 0)
  const pixelRatio = window.devicePixelRatio || 1
  const scale = (THUMBNAIL_WIDTH * pixelRatio) / page.width
  const tile = { x: 0, y: 0, width: Math.ceil(page.width * scale), height: Math.ceil(page.height * scale) }
  useTile(canvas, nearView ? { session, pageId: page.id, revision, scale, tile } : null, 'prefetch')

  useEffect(() => {
    const element = canvas.current
    if (!element) return
    const observer = new IntersectionObserver(([entry]) => setNearView(entry.isIntersecting), { rootMargin: '400px' })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (current) button.current?.scrollIntoView({ block: 'nearest' })
  }, [current])

  function hover(event: DragEvent<HTMLButtonElement>) {
    const bounds = event.currentTarget.getBoundingClientRect()
    onHover(event.clientY > bounds.top + bounds.height / 2)
  }

  function choose(event: MouseEvent) {
    onChoose({ toggle: event.metaKey || event.ctrlKey, range: event.shiftKey })
  }

  return (
    <button
      ref={button}
      type="button"
      className={`group/thumb flex cursor-pointer flex-col items-center gap-1.5 p-0 text-xs text-ink ${dropBefore ? 'shadow-[-6px_0_0_-3px_var(--color-selection)]' : ''}`}
      onClick={choose}
      draggable
      onDragStart={onDragStart}
      onDragOver={hover}
      aria-label={`Page ${number}`}
      aria-current={current}
      aria-pressed={selected}
    >
      <canvas
        ref={canvas}
        className={`rounded-lg border bg-white outline-offset-2 ${selected ? 'border-selection outline-3 outline-selection-fill' : current ? 'border-edit-outline outline-1 outline-transparent' : 'border-separator outline-1 outline-transparent group-hover/thumb:border-menu-border'}`}
        style={{ width: THUMBNAIL_WIDTH, height: (THUMBNAIL_WIDTH * page.height) / page.width }} />
      <span className={selected ? 'font-bold text-accent-ink' : ''}>{number}</span>
    </button>
  )
}
