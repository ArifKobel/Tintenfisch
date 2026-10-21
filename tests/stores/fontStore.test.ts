import { beforeEach, describe, expect, it, vi } from 'vitest'
import { parseCatalog } from '@/services/fontCatalog.ts'
import { useEditingStore } from '@/stores/editingStore.ts'
import { useFontStore } from '@/stores/fontStore.ts'
import type { PageFont } from '@/types/protocol.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const download = vi.hoisted(() => vi.fn(async (url: string) => new TextEncoder().encode(url)))

vi.mock('@/services/fontCatalog.ts', async (importOriginal) => {
  const catalog = await importOriginal<{ parseCatalog: typeof parseCatalog }>()
  const families = catalog.parseCatalog({ fontsource: [], fontshare: [['cabinet-grotesk', 'Cabinet Grotesk', 'sans', [[700, false, 'bold']]]] })
  return { ...catalog, catalogFamilies: async () => families, downloadFont: download }
})

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

let port: FakeEnginePort
let memory: Map<string, string>
const refresh = vi.fn(async () => {})

beforeEach(() => {
  port = FakeEnginePort.attach()
  memory = new Map()
  vi.stubGlobal('localStorage', { getItem: (key: string) => memory.get(key) ?? null, setItem: (key: string, value: string) => void memory.set(key, value) })
  download.mockClear()
  refresh.mockClear()
  useEditingStore.setState({ refresh })
})

const fonts = () => useFontStore.getState()
const show = (shown: PageFont[]) => useEditingStore.setState({ fonts: shown })

describe('fontStore', () => {
  it('answers at once when downloads are allowed', async () => {
    useFontStore.setState({ downloads: 'allowed' })
    expect(await fonts().request()).toBe(true)
    expect(fonts().asking).toBe(false)
  })

  it('asks again on request even after a refusal and waits for the answer', async () => {
    useFontStore.setState({ downloads: 'refused' })
    const answer = fonts().request()
    expect(fonts()).toMatchObject({ downloads: 'refused', asking: true })
    fonts().answer(true)
    expect(await answer).toBe(true)
    expect(memory.get('tintenfisch.fontDownloads')).toBe('allowed')
  })

  it('asks once before downloading anything', async () => {
    show([cabinet])
    await Promise.resolve()
    expect(fonts()).toMatchObject({ downloads: 'unknown', asking: true })
    expect(download).not.toHaveBeenCalled()
  })

  it('stays offline when downloads were refused', async () => {
    useFontStore.setState({ downloads: 'refused' })
    show([cabinet])
    await Promise.resolve()
    expect(fonts().asking).toBe(false)
    expect(download).not.toHaveBeenCalled()
  })

  it('downloads the matching font once allowed and hands it to the engine', async () => {
    show([cabinet, { ...cabinet, name: 'F2', baseFont: 'Helvetica', embedded: false }])
    fonts().answer(true)
    await vi.waitFor(() => expect(port.requests).toHaveLength(1))
    expect(download).toHaveBeenCalledWith('https://cdn.fontshare.com/wf/bold.ttf')
    expect(port.requests[0]).toMatchObject({ kind: 'registerFontReplacement', key: 'AAAAAA+false' })
    await port.answerNext(undefined)
    await vi.waitFor(() => expect(refresh).toHaveBeenCalledOnce())
    show([cabinet])
    await Promise.resolve()
    expect(download).toHaveBeenCalledOnce()
  })

  it('loads a chosen catalog font once in the weight closest to the text', async () => {
    useFontStore.setState({ downloads: 'allowed' })
    const [family] = parseCatalog({ fontsource: [['inter', 'Inter', 'sans-serif', [400, 700], false, false]], fontshare: [] })
    const choosing = fonts().catalogFont(family, 700, false)
    await vi.waitFor(() => expect(port.requests.at(-1)).toMatchObject({ kind: 'registerFontReplacement', key: 'fontsource:inter:700:normal' }))
    await port.answerNext(undefined)
    expect(await choosing).toEqual({ kind: 'catalog', key: 'fontsource:inter:700:normal', source: 'fontsource', id: 'inter', family: 'Inter', weight: 700, italic: false })
    expect(download).toHaveBeenCalledWith('https://cdn.jsdelivr.net/fontsource/fonts/inter@latest/latin-700-normal.ttf')
    const requests = port.requests.length
    await fonts().catalogFont(family, 700, false)
    expect(port.requests).toHaveLength(requests)
  })
})
