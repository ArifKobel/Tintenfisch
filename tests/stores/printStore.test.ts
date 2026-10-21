import { describe, expect, it, vi } from 'vitest'
import type { RenderedTile, Request } from '@/types/protocol.ts'
import { usePrintStore } from '@/stores/printStore.ts'
import { useViewportStore } from '@/stores/viewportStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const pages = [
  { id: '1-0', width: 612, height: 792, rotate: 0 },
  { id: '2-0', width: 792, height: 612, rotate: 90 },
  { id: '3-0', width: 300, height: 400, rotate: 0 },
]

const encoded = vi.hoisted((): RenderedTile[] => [])

vi.mock('@/services/CanvasPictureEncoder.ts', () => ({
  CanvasPictureEncoder: {
    encode: async (tile: RenderedTile) => {
      encoded.push(tile)
      return new Blob([tile.rgba.slice(0, 1)], { type: 'image/png' })
    },
  },
}))

async function createStore() {
  const port = FakeEnginePort.attach()
  const documents = useDocumentStore.getState()
  const opening = documents.open('book.pdf', new Uint8Array(4))
  await port.answerNext({ status: 'opened', version: '1.7', byteLength: 4, objectCount: 9, pageCount: 3, repaired: false })
  await port.answerNext(pages)
  await opening
  const viewport = useViewportStore.getState()
  encoded.length = 0
  const printing = usePrintStore.getState()
  return { port, viewport, printing, encoded }
}

const tile = (width: number, height: number) => ({ width, height, rgba: new Uint8Array(4) })

async function answerTiles(port: FakeEnginePort, count: number) {
  for (let index = 0; index < count; index++) {
    const request = port.requests[port.requests.length - 1] as Extract<Request, { kind: 'renderTile' }>
    await port.answerNext(tile(request.width, request.height))
    await Promise.resolve()
    await Promise.resolve()
  }
}

describe('printStore', () => {
  it('renders every page whole at the chosen resolution and hands over a sheet in their sizes', async () => {
    const { port, printing, encoded } = await createStore()
    printing.openDialog()
    printing.change({ resolution: 150 })
    const job = printing.print()
    await Promise.resolve()
    expect(port.requests.at(-1)).toMatchObject({ kind: 'renderTile', pageId: '1-0', scale: 150 / 72, x: 0, y: 0, width: 1275, height: 1650 })
    await answerTiles(port, 3)
    await job
    expect(port.requests.filter((request) => request.kind === 'renderTile').map((request) => request.pageId)).toEqual(['1-0', '2-0', '3-0'])
    expect(encoded.map((rendered) => [rendered.width, rendered.height])).toEqual([
      [1275, 1650],
      [1650, 1275],
      [625, 833],
    ])
    const { sheet, dialogOpen, progress } = usePrintStore.getState()
    expect(dialogOpen).toBe(false)
    expect(progress).toBeNull()
    expect(sheet?.map((page) => [page.width, page.height])).toEqual([
      [612, 792],
      [792, 612],
      [300, 400],
    ])
    printing.finish()
    expect(usePrintStore.getState().sheet).toBeNull()
  })

  it('prints the current page or a range, and says which pages it can print', async () => {
    const { port, viewport, printing } = await createStore()
    viewport.setCurrentPage('2-0')
    printing.change({ choice: 'current' })
    const current = printing.print()
    await Promise.resolve()
    expect(port.requests.at(-1)).toMatchObject({ kind: 'renderTile', pageId: '2-0' })
    await answerTiles(port, 1)
    await current
    printing.finish()
    printing.change({ choice: 'range', range: '3, 1' })
    const range = printing.print()
    await Promise.resolve()
    await answerTiles(port, 2)
    await range
    expect(usePrintStore.getState().sheet).toHaveLength(2)
    expect(port.requests.slice(-2).map((request) => (request.kind === 'renderTile' ? request.pageId : ''))).toEqual(['3-0', '1-0'])
    printing.change({ range: '4' })
    await printing.print()
    expect(usePrintStore.getState().notice).toBe('Enter pages between 1 and 3, like 1-3, 5')
  })
})
