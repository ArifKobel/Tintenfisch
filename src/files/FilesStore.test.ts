import { IDBFactory } from 'fake-indexeddb'
import { describe, expect, it, vi } from 'vitest'
import { DocumentStore } from '@document/DocumentStore.ts'
import { EngineClient } from '@engine/EngineClient.ts'
import { AutoSave } from '@files/AutoSave.ts'
import { DocumentLibrary } from '@files/DocumentLibrary.ts'
import { FilesStore } from '@files/FilesStore.ts'
import { Router, type BrowserWindow } from '@files/Router.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'

const opened = { status: 'opened', version: '1.7', byteLength: 1, objectCount: 3, pageCount: 1, repaired: false }
const pages = [{ id: '3-0', width: 612, height: 792, rotate: 0 }]

function setUp(path: string, library = new DocumentLibrary(new IDBFactory())) {
  const location = { pathname: path }
  const history = {
    pushState: (_state: unknown, _title: string, url: string) => (location.pathname = url),
    replaceState: (_state: unknown, _title: string, url: string) => (location.pathname = url),
  }
  const window = { location, history, addEventListener: () => {} } as unknown as BrowserWindow
  const port = new FakeEnginePort()
  const client = new EngineClient(port)
  const documents = new DocumentStore(client)
  const router = new Router(window)
  const files = new FilesStore(library, documents, router, new AutoSave(client, documents, library, () => () => {}))
  const answerOpening = async () => {
    await vi.waitFor(() => expect(port.requests.at(-1)?.kind).toBe('open'))
    await port.answerNext(opened)
    await vi.waitFor(() => expect(port.requests.at(-1)?.kind).toBe('pages'))
    await port.answerNext(pages)
    await vi.waitFor(() => expect(documents.getSnapshot().phase).toBe('ready'))
  }
  return { location, port, documents, library, files, answerOpening }
}

describe('FilesStore', () => {
  it('keeps a newly opened file and gives it its own address', async () => {
    const { location, documents, library, files, answerOpening } = setUp('/')
    const opening = files.openNew('report.pdf', new Uint8Array([1, 2]))
    await answerOpening()
    await opening
    const [stored] = await library.list()
    expect(stored).toMatchObject({ name: 'report.pdf', size: 2 })
    expect(location.pathname).toBe(`/documents/${stored.id}`)
    expect(files.getSnapshot().current).toBe(stored.id)
    expect(documents.getSnapshot()).toMatchObject({ phase: 'ready', name: 'report.pdf' })
  })

  it('opens a stored file from its address', async () => {
    const library = new DocumentLibrary(new IDBFactory())
    const file = await library.add('stored.pdf', new Uint8Array([7]))
    const { port, documents, files, answerOpening } = setUp(`/documents/${file.id}`, library)
    const starting = files.start()
    await answerOpening()
    await starting
    expect(port.requests[0]).toMatchObject({ kind: 'open', bytes: new Uint8Array([7]) })
    expect(documents.getSnapshot()).toMatchObject({ phase: 'ready', name: 'stored.pdf' })
  })

  it('sends unknown addresses home with a hint', async () => {
    const { location, port, files } = setUp('/documents/unknown')
    await files.start()
    expect(files.getSnapshot().missing).toBe(true)
    expect(location.pathname).toBe('/')
    expect(port.requests).toEqual([])
  })

  it('closes the document on the way home and forgets a deleted current file', async () => {
    const { location, documents, library, files, answerOpening } = setUp('/')
    const opening = files.openNew('report.pdf', new Uint8Array([1]))
    await answerOpening()
    await opening
    const [stored] = await library.list()
    files.askToDelete(stored)
    await files.confirmDelete()
    expect(location.pathname).toBe('/')
    expect(documents.getSnapshot().phase).toBe('empty')
    expect(files.getSnapshot()).toMatchObject({ files: [], current: null, deleting: null })
  })

  it('keeps a reported problem until the next file opens', async () => {
    const { files, answerOpening } = setUp('/')
    files.report('Only PDF files can be opened.')
    expect(files.getSnapshot().notice).toBe('Only PDF files can be opened.')
    const opening = files.openNew('report.pdf', new Uint8Array([1]))
    await answerOpening()
    await opening
    expect(files.getSnapshot().notice).toBeNull()
  })
})
