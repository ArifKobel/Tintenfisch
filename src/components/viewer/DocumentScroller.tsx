import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { PageSlot } from '@/components/viewer/PageSlot.tsx'
import { CSS_PIXELS_PER_POINT, PAGE_GAP, PageLayout } from '@/lib/PageLayout.ts'
import type { ReadyDocument } from '@/stores/documentStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'

const WHEEL_ZOOM_SPEED = 1 / 300

interface Scroll {
  top: number
  left: number
  width: number
  height: number
}

export function DocumentScroller({ document }: { document: ReadyDocument }) {
  const { zoom, focusRequest, reportAvailableWidth, fitWidth, setZoom, reportViewTop, setCurrentPage } = useViewportStore()
  const container = useRef<HTMLDivElement>(null)
  const [scroll, setScroll] = useState<Scroll>({ top: 0, left: 0, width: 0, height: 0 })
  const layout = useMemo(() => new PageLayout(document.pages, zoom), [document.pages, zoom])
  const previousLayout = useRef(layout)
  const shownFocus = useRef(0)
  const pixelRatio = window.devicePixelRatio || 1
  const widestAtZoomOne = Math.max(...document.pages.map((page) => page.width)) * CSS_PIXELS_PER_POINT

  useLayoutEffect(() => {
    const element = container.current
    if (!element) return
    const measure = () =>
      setScroll({ top: element.scrollTop, left: element.scrollLeft, width: element.clientWidth, height: element.clientHeight })
    const observer = new ResizeObserver(() => {
      reportAvailableWidth(element.clientWidth - 2 * PAGE_GAP)
      measure()
    })
    observer.observe(element)
    let frame = 0
    const onScroll = () => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(measure)
    }
    element.addEventListener('scroll', onScroll, { passive: true })
    return () => {
      observer.disconnect()
      element.removeEventListener('scroll', onScroll)
      cancelAnimationFrame(frame)
    }
  }, [reportAvailableWidth])

  useLayoutEffect(() => {
    const available = (container.current?.clientWidth ?? 0) - 2 * PAGE_GAP
    reportAvailableWidth(available)
    if (available < widestAtZoomOne) fitWidth(widestAtZoomOne)
    else setZoom(1)
  }, [reportAvailableWidth, fitWidth, setZoom, document.session, widestAtZoomOne])

  useLayoutEffect(() => {
    const element = container.current
    const previous = previousLayout.current
    previousLayout.current = layout
    if (!element || previous === layout || element.scrollTop === 0) return
    const anchor = previous.anchorAt(element.scrollTop + element.clientHeight / 2)
    element.scrollTop = layout.offsetOf(anchor) - element.clientHeight / 2
  }, [layout])

  useEffect(() => {
    const element = container.current
    if (!element) return
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey) return
      event.preventDefault()
      setZoom(useViewportStore.getState().zoom * Math.exp(-event.deltaY * WHEEL_ZOOM_SPEED))
    }
    element.addEventListener('wheel', onWheel, { passive: false })
    return () => element.removeEventListener('wheel', onWheel)
  }, [setZoom])

  useEffect(() => {
    if (!focusRequest || !container.current || focusRequest.sequence === shownFocus.current) return
    const placement = layout.placements.find((candidate) => candidate.id === focusRequest.pageId)
    if (!placement) return
    shownFocus.current = focusRequest.sequence
    const offset = focusRequest.y * zoom * CSS_PIXELS_PER_POINT
    const lead = focusRequest.atTop ? 0 : container.current.clientHeight / 3
    container.current.scrollTop = placement.top - PAGE_GAP + Math.max(0, offset - lead)
  }, [focusRequest, layout, zoom])

  const topPlacement = layout.placements[layout.anchorAt(scroll.top).index]
  useEffect(() => {
    if (topPlacement) reportViewTop(topPlacement.id, Math.max(0, scroll.top + PAGE_GAP - topPlacement.top) / (zoom * CSS_PIXELS_PER_POINT))
  }, [reportViewTop, topPlacement, scroll.top, zoom])

  const center = layout.placements[layout.anchorAt(scroll.top + scroll.height / 2).index]
  useEffect(() => {
    if (center) setCurrentPage(center.id)
  }, [setCurrentPage, center])

  const contentWidth = Math.max(scroll.width, layout.widestPage + 2 * PAGE_GAP)
  const visible = layout.visible(scroll.top - scroll.height, scroll.top + 2 * scroll.height)
  const scale = zoom * CSS_PIXELS_PER_POINT * pixelRatio
  return (
    <div ref={container} className="flex-1 overflow-auto">
      <div className="relative" style={{ height: layout.totalHeight, width: contentWidth }}>
        {visible.map((placement) => {
          const left = Math.max(PAGE_GAP, (contentWidth - placement.width) / 2)
          const visibleArea = {
            left: (scroll.left - left) * pixelRatio,
            top: (scroll.top - placement.top) * pixelRatio,
            right: (scroll.left + scroll.width - left) * pixelRatio,
            bottom: (scroll.top + scroll.height - placement.top) * pixelRatio,
          }
          return (
            <PageSlot
              key={placement.id}
              session={document.session}
              page={document.pages[placement.index]}
              placement={placement}
              left={left}
              scale={scale}
              cssScale={zoom * CSS_PIXELS_PER_POINT}
              pixelRatio={pixelRatio}
              visibleArea={visibleArea}
            />
          )
        })}
      </div>
    </div>
  )
}
