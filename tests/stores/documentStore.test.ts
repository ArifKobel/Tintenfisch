import { beforeEach, describe, expect, it } from 'vitest'
import { onDocumentReplaced, useDocumentStore, type DocumentState } from '@/stores/documentStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const opened = { version: '1.7', byteLength: 4, objectCount: 9, pageCount: 1, repaired: false }
const openedResult = { status: 'opened', ...opened }
const pages = [{ id: '3-0', width: 612, height: 792, rotate: 0 }]

let port: FakeEnginePort

beforeEach(() => {
  port = FakeEnginePort.attach()
})

const documents = () => useDocumentStore.getState()

async function openSample(name = 'sample.pdf') {
  const opening = documents().open(name, new Uint8Array(4))
  await port.answerNext(openedResult)
  await port.answerNext(pages)
  await opening
}

describe('documentStore', () => {
  it('opens, reads the pages and becomes ready with a fresh session', async () => {
    const phases: DocumentState['phase'][] = []
    const stop = useDocumentStore.subscribe(({ document }) => phases.push(document.phase))
    await openSample()
    stop()
    expect(phases).toEqual(['loading', 'ready'])
    expect(port.requests.map((request) => request.kind)).toEqual(['open', 'pages'])
    expect(documents().document).toEqual({ phase: 'ready', name: 'sample.pdf', session: 1, pages, ...opened })
    await openSample('second.pdf')
    expect(documents().document).toMatchObject({ name: 'second.pdf', session: 2 })
  })

  it('keeps the failure message when the engine rejects the file', async () => {
    const opening = documents().open('notes.txt', new Uint8Array(4))
    port.answer({ id: port.lastId, ok: false, error: 'not a PDF file' })
    await opening
    expect(documents().document).toEqual({ phase: 'failed', name: 'notes.txt', message: 'not a PDF file' })
  })

  it('asks the engine for the saved bytes of an open document', async () => {
    await openSample()
    const saving = documents().save()
    expect(port.requests.at(-1)).toMatchObject({ kind: 'save', optimize: false })
    port.answer({ id: port.lastId, ok: true, result: new Uint8Array([1, 2]), revision: '' })
    expect(await saving).toEqual(new Uint8Array([1, 2]))
  })

  it('refuses to save before a document is open', async () => {
    await expect(documents().save()).rejects.toThrow('no document is open')
    expect(port.sent).toHaveLength(0)
  })

  it('asks for a password, reports a wrong one and opens with the right one', async () => {
    const opening = documents().open('locked.pdf', new Uint8Array(4))
    await port.answerNext({ status: 'locked', wrongPassword: false })
    await opening
    expect(documents().document).toEqual({ phase: 'locked', name: 'locked.pdf', wrongPassword: false })
    const guessing = documents().unlock('guess')
    expect(port.requests.at(-1)).toMatchObject({ kind: 'unlock', password: 'guess' })
    await port.answerNext({ status: 'locked', wrongPassword: true })
    await guessing
    expect(documents().document).toMatchObject({ phase: 'locked', wrongPassword: true })
    const unlocking = documents().unlock('secret')
    await port.answerNext(openedResult)
    await port.answerNext(pages)
    await unlocking
    expect(documents().document).toMatchObject({ phase: 'ready', name: 'locked.pdf', pageCount: 1 })
  })

  it('replaces the pages of the open document without counting it as a new one', async () => {
    let replaced = 0
    const stop = onDocumentReplaced(() => replaced++)
    await openSample()
    expect(replaced).toBe(2)
    const turned = [{ ...pages[0], rotate: 90 }]
    documents().replacePages(turned)
    expect(documents().document).toMatchObject({ session: 1, pages: turned })
    const refreshing = documents().refreshPages()
    await port.answerNext(pages)
    await refreshing
    expect(documents().document).toMatchObject({ session: 1, pages })
    expect(replaced).toBe(2)
    stop()
  })

  it('advances only the touched pages', () => {
    documents().touchPages(['3-0', '3-0', '7-0'])
    expect(documents().pageRevisions).toEqual({ '3-0': 2, '7-0': 1 })
  })
})
