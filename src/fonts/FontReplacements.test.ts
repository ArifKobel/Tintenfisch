import { describe, expect, it, vi } from 'vitest'
import type { EditingStore } from '@editing/EditingStore.ts'
import { EngineClient } from '@engine/EngineClient.ts'
import type { PageFont } from '@engine/protocol.ts'
import { FontCatalog } from '@fonts/FontCatalog.ts'
import { FontConsent } from '@fonts/FontConsent.ts'
import { FontReplacements } from '@fonts/FontReplacements.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'

const cabinet: PageFont = {
  name: 'F1',
  font: 'Cabinet Grotesk Bold',
  baseFont: 'AAAAAA+false',
  embedded: true,
  family: 'sans',
  bold: true,
  italic: false,
  weight: 700,
}

function setUp(stored: string | null) {
  const port = new FakeEnginePort()
  const listeners = new Set<() => void>()
  let fonts: PageFont[] = []
  const refresh = vi.fn(async () => {})
  const editing = {
    getSnapshot: () => ({ fonts }),
    subscribe: (listener: () => void) => listeners.add(listener),
    refresh,
  } as unknown as EditingStore
  const memory = new Map(stored ? [['tintenfisch.fontDownloads', stored]] : [])
  const consent = new FontConsent({ getItem: (key) => memory.get(key) ?? null, setItem: (key, value) => void memory.set(key, value) })
  const catalog = new FontCatalog(async () => ({ fontsource: [], fontshare: [['cabinet-grotesk', 'Cabinet Grotesk', 'sans', [[700, false, 'bold']]]] }))
  const download = vi.fn(async (url: string) => new TextEncoder().encode(url))
  new FontReplacements(new EngineClient(port), editing, consent, catalog, download)
  const show = (shown: PageFont[]) => {
    fonts = shown
    listeners.forEach((listener) => listener())
  }
  return { port, consent, download, refresh, show, memory }
}

describe('FontReplacements', () => {
  it('asks once before downloading anything', async () => {
    const { consent, download, show } = setUp(null)
    show([cabinet])
    await Promise.resolve()
    expect(consent.getSnapshot()).toEqual({ downloads: 'unknown', asking: true })
    expect(download).not.toHaveBeenCalled()
  })

  it('downloads the matching font once allowed and hands it to the engine', async () => {
    const { port, consent, download, refresh, show, memory } = setUp(null)
    show([cabinet, { ...cabinet, name: 'F2', baseFont: 'Helvetica', embedded: false }])
    consent.answer(true)
    await vi.waitFor(() => expect(port.requests).toHaveLength(1))
    expect(download).toHaveBeenCalledWith('https://cdn.fontshare.com/wf/bold.ttf')
    expect(port.requests[0]).toMatchObject({ kind: 'registerFontReplacement', key: 'AAAAAA+false' })
    await port.answerNext(undefined)
    await vi.waitFor(() => expect(refresh).toHaveBeenCalledOnce())
    expect(memory.get('tintenfisch.fontDownloads')).toBe('allowed')
    show([cabinet])
    await Promise.resolve()
    expect(download).toHaveBeenCalledOnce()
  })

  it('stays offline when downloads were refused', async () => {
    const { consent, download, show } = setUp('refused')
    show([cabinet])
    await Promise.resolve()
    expect(consent.getSnapshot().asking).toBe(false)
    expect(download).not.toHaveBeenCalled()
  })

  it('loads a chosen catalog font once in the weight closest to the text', async () => {
    const { port, consent, download } = setUp('allowed')
    const replacements = new FontReplacements(new EngineClient(port), { getSnapshot: () => ({ fonts: [] }), subscribe: () => {} } as unknown as EditingStore, consent, new FontCatalog(async () => ({ fontsource: [], fontshare: [] })), download)
    const [family] = FontCatalog.parse({ fontsource: [['inter', 'Inter', 'sans-serif', [400, 700], false, false]], fontshare: [] })
    const choosing = replacements.catalogFont(family, 700, false)
    await vi.waitFor(() => expect(port.requests.at(-1)).toMatchObject({ kind: 'registerFontReplacement', key: 'fontsource:inter:700:normal' }))
    await port.answerNext(undefined)
    expect(await choosing).toEqual({ kind: 'catalog', key: 'fontsource:inter:700:normal', source: 'fontsource', id: 'inter', family: 'Inter', weight: 700, italic: false })
    expect(download).toHaveBeenCalledWith('https://cdn.jsdelivr.net/fontsource/fonts/inter@latest/latin-700-normal.ttf')
    const requests = port.requests.length
    await replacements.catalogFont(family, 700, false)
    expect(port.requests).toHaveLength(requests)
  })
})
