import { describe, expect, it } from 'vitest'
import { DocumentStore, type DocumentState } from '@/stores/DocumentStore.ts'
import { EngineClient } from '@/services/EngineClient.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const opened = { version: '1.7', byteLength: 4, objectCount: 9, pageCount: 1, repaired: false }
const openedResult = { status: 'opened', ...opened }
const pages = [{ id: '3-0', width: 612, height: 792, rotate: 0 }]

function createStore() {
  const port = new FakeEnginePort()
  const store = new DocumentStore(new EngineClient(port))
  const phases: DocumentState['phase'][] = []
  store.subscribe(() => phases.push(store.getSnapshot().phase))
  return { port, store, phases }
}

async function openSample(port: FakeEnginePort, store: DocumentStore, name = 'sample.pdf') {
  const opening = store.open(name, new Uint8Array(4))
  await port.answerNext(openedResult)
  await port.answerNext(pages)
  await opening
}

describe('DocumentStore', () => {
  it('opens, reads the pages and becomes ready with a fresh session', async () => {
    const { port, store, phases } = createStore()
    await openSample(port, store)
    expect(phases).toEqual(['loading', 'ready'])
    expect(port.requests.map((request) => request.kind)).toEqual(['open', 'pages'])
    expect(store.getSnapshot()).toEqual({ phase: 'ready', name: 'sample.pdf', session: 1, pages, ...opened })
    await openSample(port, store, 'second.pdf')
    expect(store.getSnapshot()).toMatchObject({ name: 'second.pdf', session: 2 })
  })

  it('keeps the failure message when the engine rejects the file', async () => {
    const { port, store } = createStore()
    const opening = store.open('notes.txt', new Uint8Array(4))
    port.answer({ id: port.lastId, ok: false, error: 'not a PDF file' })
    await opening
    expect(store.getSnapshot()).toEqual({ phase: 'failed', name: 'notes.txt', message: 'not a PDF file' })
  })

  it('stops notifying a listener once it unsubscribes', async () => {
    const { port, store } = createStore()
    let calls = 0
    const unsubscribe = store.subscribe(() => calls++)
    unsubscribe()
    await openSample(port, store)
    expect(calls).toBe(0)
  })

  it('asks the engine for the saved bytes of an open document', async () => {
    const { port, store } = createStore()
    await openSample(port, store)
    const saving = store.save()
    expect(port.requests[port.requests.length - 1]).toMatchObject({ kind: 'save', optimize: false })
    port.answer({ id: port.lastId, ok: true, result: new Uint8Array([1, 2]), revision: '' })
    expect(await saving).toEqual(new Uint8Array([1, 2]))
  })

  it('refuses to save before a document is open', async () => {
    const { store, port } = createStore()
    await expect(store.save()).rejects.toThrow('no document is open')
    expect(port.sent).toHaveLength(0)
  })

  it('asks for a password, reports a wrong one and opens with the right one', async () => {
    const { port, store } = createStore()
    const opening = store.open('locked.pdf', new Uint8Array(4))
    await port.answerNext({ status: 'locked', wrongPassword: false })
    await opening
    expect(store.getSnapshot()).toEqual({ phase: 'locked', name: 'locked.pdf', wrongPassword: false })
    const guessing = store.unlock('guess')
    expect(port.requests[port.requests.length - 1]).toMatchObject({ kind: 'unlock', password: 'guess' })
    await port.answerNext({ status: 'locked', wrongPassword: true })
    await guessing
    expect(store.getSnapshot()).toMatchObject({ phase: 'locked', wrongPassword: true })
    const unlocking = store.unlock('secret')
    await port.answerNext(openedResult)
    await port.answerNext(pages)
    await unlocking
    expect(store.getSnapshot()).toMatchObject({ phase: 'ready', name: 'locked.pdf', pageCount: 1 })
  })
  it('replaces the pages of the open document without counting it as a new one', async () => {
    const { port, store } = createStore()
    let replaced = 0
    store.onDocumentReplaced(() => replaced++)
    await openSample(port, store)
    expect(replaced).toBe(2)
    const turned = [{ ...pages[0], rotate: 90 }]
    store.replacePages(turned)
    expect(store.getSnapshot()).toMatchObject({ session: 1, pages: turned })
    const refreshing = store.refreshPages()
    await port.answerNext(pages)
    await refreshing
    expect(store.getSnapshot()).toMatchObject({ session: 1, pages })
    expect(replaced).toBe(2)
  })
})
