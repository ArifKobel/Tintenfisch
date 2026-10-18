export const ZOOM_STEPS = [0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 5, 6.4, 8, 12, 16, 24, 32]
export const MIN_ZOOM = ZOOM_STEPS[0]
export const MAX_ZOOM = ZOOM_STEPS[ZOOM_STEPS.length - 1]

export interface ViewportState {
  zoom: number
  currentPageId: string | null
  focusRequest: { pageId: string; y: number; atTop: boolean; sequence: number } | null
}

type Listener = () => void

export class ViewportStore {
  readonly #listeners = new Set<Listener>()
  #state: ViewportState = { zoom: 1, currentPageId: null, focusRequest: null }
  #sequence = 0
  #availableWidth = 0
  #viewTop: { pageId: string; y: number } | null = null

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): ViewportState => this.#state

  setZoom(zoom: number) {
    const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom))
    if (clamped !== this.#state.zoom) this.#update({ ...this.#state, zoom: clamped })
  }

  zoomIn() {
    this.setZoom(ZOOM_STEPS.find((step) => step > this.#state.zoom + 1e-9) ?? MAX_ZOOM)
  }

  zoomOut() {
    this.setZoom([...ZOOM_STEPS].reverse().find((step) => step < this.#state.zoom - 1e-9) ?? MIN_ZOOM)
  }

  reportAvailableWidth(width: number) {
    this.#availableWidth = width
  }

  fitWidth(pageWidthAtZoomOne: number) {
    if (pageWidthAtZoomOne > 0 && this.#availableWidth > 0) this.setZoom(this.#availableWidth / pageWidthAtZoomOne)
  }

  setCurrentPage(pageId: string) {
    if (pageId !== this.#state.currentPageId) this.#update({ ...this.#state, currentPageId: pageId })
  }

  get viewTop(): { pageId: string; y: number } | null {
    return this.#viewTop
  }

  reportViewTop(pageId: string, y: number) {
    this.#viewTop = { pageId, y }
  }

  focusPage(pageId: string, y = 0) {
    this.#update({ ...this.#state, focusRequest: { pageId, y, atTop: false, sequence: ++this.#sequence } })
  }

  showAtTop(pageId: string, y = 0) {
    this.#update({ ...this.#state, focusRequest: { pageId, y, atTop: true, sequence: ++this.#sequence } })
  }

  #update(state: ViewportState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
