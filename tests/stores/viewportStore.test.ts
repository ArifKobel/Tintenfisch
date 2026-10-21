import { describe, expect, it, vi } from 'vitest'
import { MAX_ZOOM, useViewportStore } from '@/stores/viewportStore.ts'

const viewport = () => useViewportStore.getState()

describe('viewportStore', () => {
  it('steps through the zoom levels and clamps at the ends', () => {
    viewport().zoomIn()
    expect(viewport().zoom).toBe(1.1)
    viewport().setZoom(1.12)
    viewport().zoomOut()
    expect(viewport().zoom).toBe(1.1)
    viewport().setZoom(1000)
    expect(viewport().zoom).toBe(MAX_ZOOM)
  })

  it('fits the widest page into the reported width', () => {
    viewport().fitWidth(800)
    expect(viewport().zoom).toBe(1)
    viewport().reportAvailableWidth(1200)
    viewport().fitWidth(800)
    expect(viewport().zoom).toBe(1.5)
  })

  it('keeps the page fitted while the width changes until zoomed by hand', () => {
    viewport().reportAvailableWidth(1200)
    viewport().fitWidth(800)
    viewport().reportAvailableWidth(800)
    expect(viewport().zoom).toBe(1)
    viewport().zoomIn()
    viewport().reportAvailableWidth(1600)
    expect(viewport().zoom).toBe(1.1)
  })

  it('numbers focus requests so focusing the same page twice still scrolls', () => {
    viewport().focusPage('3-0')
    const first = viewport().focusRequest
    viewport().focusPage('3-0')
    expect(viewport().focusRequest?.sequence).toBe((first?.sequence ?? 0) + 1)
  })

  it('reveals a page target at the top and opens only web and mail addresses', () => {
    const open = vi.fn()
    vi.stubGlobal('window', { open })
    viewport().reveal({ kind: 'page', pageId: '4-0', top: 120 })
    expect(viewport().focusRequest).toMatchObject({ pageId: '4-0', y: 120, atTop: true })
    viewport().reveal({ kind: 'page', pageId: '5-0', top: null })
    expect(viewport().focusRequest).toMatchObject({ pageId: '5-0', y: 0 })
    viewport().reveal({ kind: 'uri', uri: ' https://example.org/ ' })
    viewport().reveal({ kind: 'uri', uri: 'mailto:someone@example.org' })
    viewport().reveal({ kind: 'uri', uri: 'javascript:alert(1)' })
    viewport().reveal({ kind: 'uri', uri: 'file:///etc/passwd' })
    expect(open.mock.calls.map(([uri]) => uri)).toEqual(['https://example.org/', 'mailto:someone@example.org'])
    vi.unstubAllGlobals()
  })

  it('names the view at the top of the window as a target', () => {
    viewport().reportViewTop('3-0', 240)
    expect(viewport().currentView()).toEqual({ kind: 'page', pageId: '3-0', top: 240 })
  })
})
