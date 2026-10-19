import { describe, expect, it } from 'vitest'
import { MAX_ZOOM, ViewportStore } from '@viewer/ViewportStore.ts'

describe('ViewportStore', () => {
  it('steps through the zoom levels and clamps at the ends', () => {
    const viewport = new ViewportStore()
    viewport.zoomIn()
    expect(viewport.getSnapshot().zoom).toBe(1.1)
    viewport.setZoom(1.12)
    viewport.zoomOut()
    expect(viewport.getSnapshot().zoom).toBe(1.1)
    viewport.setZoom(1000)
    expect(viewport.getSnapshot().zoom).toBe(MAX_ZOOM)
  })

  it('fits the widest page into the reported width', () => {
    const viewport = new ViewportStore()
    viewport.fitWidth(800)
    expect(viewport.getSnapshot().zoom).toBe(1)
    viewport.reportAvailableWidth(1200)
    viewport.fitWidth(800)
    expect(viewport.getSnapshot().zoom).toBe(1.5)
  })

  it('keeps the page fitted while the width changes until zoomed by hand', () => {
    const viewport = new ViewportStore()
    viewport.reportAvailableWidth(1200)
    viewport.fitWidth(800)
    viewport.reportAvailableWidth(800)
    expect(viewport.getSnapshot().zoom).toBe(1)
    viewport.zoomIn()
    viewport.reportAvailableWidth(1600)
    expect(viewport.getSnapshot().zoom).toBe(1.1)
  })

  it('numbers focus requests so focusing the same page twice still scrolls', () => {
    const viewport = new ViewportStore()
    viewport.focusPage('3-0')
    const first = viewport.getSnapshot().focusRequest
    viewport.focusPage('3-0')
    expect(viewport.getSnapshot().focusRequest?.sequence).toBe((first?.sequence ?? 0) + 1)
  })
})
