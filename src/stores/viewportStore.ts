import { create } from 'zustand'
import type { Target } from '@/types/protocol.ts'

export const ZOOM_STEPS = [0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 5, 6.4, 8, 12, 16, 24, 32]
export const MIN_ZOOM = ZOOM_STEPS[0]
export const MAX_ZOOM = ZOOM_STEPS[ZOOM_STEPS.length - 1]

const OPENABLE = /^(https?:|mailto:)/i

export interface FocusRequest {
  pageId: string
  y: number
  atTop: boolean
  sequence: number
}

interface ViewportStore {
  zoom: number
  currentPageId: string | null
  focusRequest: FocusRequest | null
  availableWidth: number
  fittedPageWidth: number | null
  setZoom(zoom: number): void
  zoomIn(): void
  zoomOut(): void
  reportAvailableWidth(width: number): void
  fitWidth(pageWidthAtZoomOne: number): void
  setCurrentPage(pageId: string): void
  reportViewTop(pageId: string, y: number): void
  currentView(): Target | null
  focusPage(pageId: string, y?: number): void
  showAtTop(pageId: string, y?: number): void
  reveal(target: Target | null): void
}

const clamp = (zoom: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom))

export const useViewportStore = create<ViewportStore>()((set, get) => {
  let viewTop: { pageId: string; y: number } | null = null

  const apply = (zoom: number) => {
    const clamped = clamp(zoom)
    if (clamped !== get().zoom) set({ zoom: clamped })
  }

  const fit = () => {
    const { fittedPageWidth, availableWidth } = get()
    if (fittedPageWidth !== null && availableWidth > 0) apply(availableWidth / fittedPageWidth)
  }

  const focus = (pageId: string, y: number, atTop: boolean) => {
    set({ focusRequest: { pageId, y, atTop, sequence: (get().focusRequest?.sequence ?? 0) + 1 } })
  }

  return {
    zoom: 1,
    currentPageId: null,
    focusRequest: null,
    availableWidth: 0,
    fittedPageWidth: null,

    setZoom(zoom) {
      set({ fittedPageWidth: null })
      apply(zoom)
    },

    zoomIn() {
      get().setZoom(ZOOM_STEPS.find((step) => step > get().zoom + 1e-9) ?? MAX_ZOOM)
    },

    zoomOut() {
      get().setZoom([...ZOOM_STEPS].reverse().find((step) => step < get().zoom - 1e-9) ?? MIN_ZOOM)
    },

    reportAvailableWidth(width) {
      set({ availableWidth: width })
      fit()
    },

    fitWidth(pageWidthAtZoomOne) {
      if (pageWidthAtZoomOne <= 0) return
      set({ fittedPageWidth: pageWidthAtZoomOne })
      fit()
    },

    setCurrentPage(pageId) {
      if (pageId !== get().currentPageId) set({ currentPageId: pageId })
    },

    reportViewTop(pageId, y) {
      viewTop = { pageId, y }
    },

    currentView() {
      return viewTop ? { kind: 'page', pageId: viewTop.pageId, top: viewTop.y } : null
    },

    focusPage(pageId, y = 0) {
      focus(pageId, y, false)
    },

    showAtTop(pageId, y = 0) {
      focus(pageId, y, true)
    },

    reveal(target) {
      if (target?.kind === 'page') get().showAtTop(target.pageId, target.top ?? 0)
      else if (target?.kind === 'uri' && OPENABLE.test(target.uri.trim())) window.open(target.uri.trim(), '_blank', 'noopener,noreferrer')
    },
  }
})
