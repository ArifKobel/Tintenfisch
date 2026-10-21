import { describe, expect, it } from 'vitest'
import type { Request } from '@/types/protocol.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { usePagesStore } from '@/stores/pagesStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const page = (id: string, rotate = 0) => ({ id, width: 300, height: 400, rotate })
const pages = [page('1-0'), page('2-0'), page('3-0')]
const plain = { toggle: false, range: false }

async function createStore() {
  const port = FakeEnginePort.attach()
  const documents = useDocumentStore.getState()
  const opening = documents.open('book.pdf', new Uint8Array(4))
  await port.answerNext({ status: 'opened', version: '1.7', byteLength: 4, objectCount: 9, pageCount: 3, repaired: false })
  await port.answerNext(pages)
  await opening
  const organize = usePagesStore.getState()
  return { port, documents, organize }
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('pagesStore', () => {
  it('rotates the selected pages in document order and redraws them', async () => {
    const { port, organize } = await createStore()
    organize.select('3-0', plain)
    organize.select('1-0', { toggle: true, range: false })
    const rotating = organize.rotate(1)
    expect(last(port)).toMatchObject({ kind: 'rotatePages', pageIds: ['1-0', '3-0'], quarterTurns: 1 })
    const turned = [page('1-0', 90), page('2-0'), page('3-0', 90)]
    await port.answerNext({ pages: turned, dirtyPages: ['1-0', '3-0'] })
    await rotating
    expect(useDocumentStore.getState().document).toMatchObject({ pages: turned, session: 1 })
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
  })

  it('drops deleted pages from the selection', async () => {
    const { port, organize } = await createStore()
    organize.select('2-0', plain)
    const deleting = organize.deleteSelected()
    expect(last(port)).toMatchObject({ kind: 'deletePages', pageIds: ['2-0'] })
    await port.answerNext({ pages: [pages[0], pages[2]], dirtyPages: [] })
    await deleting
    expect(usePagesStore.getState().selection.size).toBe(0)
  })

  it('inserts blank pages and files after the selection', async () => {
    const { port, organize } = await createStore()
    organize.select('1-0', plain)
    const blank = organize.insertBlank()
    expect(last(port)).toMatchObject({ kind: 'insertBlankPage', before: '2-0', width: 300, height: 400 })
    await port.answerNext({ pages, dirtyPages: [] })
    await blank
    organize.select('3-0', plain)
    const inserting = organize.insertFile('more.pdf', new Uint8Array([1, 2, 3]))
    expect(last(port)).toMatchObject({ kind: 'insertDocument', password: '', before: null })
    await port.answerNext({ status: 'locked', wrongPassword: false })
    await inserting
    expect(usePagesStore.getState().lockedInsert).toEqual({ name: 'more.pdf', wrongPassword: false })
    const unlocking = organize.unlockInsert('secret')
    expect(last(port)).toMatchObject({ kind: 'insertDocument', password: 'secret', bytes: new Uint8Array([1, 2, 3]) })
    await port.answerNext({ status: 'inserted', pages: [...pages, page('9-0')], inserted: ['9-0'] })
    await unlocking
    expect(usePagesStore.getState()).toMatchObject({ lockedInsert: null })
    expect(usePagesStore.getState().selection.inOrder(['9-0'])).toEqual(['9-0'])
  })

  it('extracts the selection under a name derived from the document', async () => {
    const { port, organize } = await createStore()
    organize.select('2-0', plain)
    const extracting = organize.extract()
    expect(last(port)).toMatchObject({ kind: 'extractPages', pageIds: ['2-0'] })
    await port.answerNext(new Uint8Array([37]))
    expect(await extracting).toEqual({ name: 'book (pages).pdf', bytes: new Uint8Array([37]) })
  })

  it('shows why the engine refused', async () => {
    const { port, organize } = await createStore()
    organize.select('1-0', plain)
    organize.select('3-0', { toggle: false, range: true })
    const deleting = organize.deleteSelected()
    await Promise.resolve()
    port.answer({ id: port.lastId, ok: false, error: 'a document keeps at least one page' })
    await deleting
    expect(usePagesStore.getState().notice).toBe('a document keeps at least one page')
  })
})
