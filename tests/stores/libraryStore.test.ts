import { beforeEach, describe, expect, it, vi } from 'vitest'
import { documentLibrary } from '@/services/DocumentLibrary.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useLibraryStore } from '@/stores/libraryStore.ts'
import { useToastStore } from '@/stores/toastStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const opened = { status: 'opened', version: '1.7', byteLength: 1, objectCount: 3, pageCount: 1, repaired: false }
const pages = [{ id: '3-0', width: 612, height: 792, rotate: 0 }]

let port: FakeEnginePort

beforeEach(async () => {
  port = FakeEnginePort.attach()
  for (const file of await documentLibrary.list()) await documentLibrary.remove(file.id)
})

const library = () => useLibraryStore.getState()

async function answerOpening() {
  await vi.waitFor(() => expect(port.requests.at(-1)?.kind).toBe('open'))
  await port.answerNext(opened)
  await vi.waitFor(() => expect(port.requests.at(-1)?.kind).toBe('pages'))
  await port.answerNext(pages)
  await vi.waitFor(() => expect(useDocumentStore.getState().document.phase).toBe('ready'))
}

describe('libraryStore', () => {
  it('keeps a newly opened file and hands back its id', async () => {
    const opening = library().openNew('report.pdf', new Uint8Array([1, 2]))
    await answerOpening()
    const id = await opening
    const [stored] = await documentLibrary.list()
    expect(stored).toMatchObject({ id, name: 'report.pdf', size: 2 })
    expect(library().current).toBe(stored.id)
    expect(useDocumentStore.getState().document).toMatchObject({ phase: 'ready', name: 'report.pdf' })
  })

  it('opens a stored file by its id', async () => {
    const file = await documentLibrary.add('stored.pdf', new Uint8Array([7]))
    const showing = library().show(file.id)
    await answerOpening()
    expect(await showing).toBe(true)
    expect(port.requests[0]).toMatchObject({ kind: 'open', bytes: new Uint8Array([7]) })
    expect(useDocumentStore.getState().document).toMatchObject({ phase: 'ready', name: 'stored.pdf' })
  })

  it('reports unknown ids as missing', async () => {
    expect(await library().show('unknown')).toBe(false)
    expect(library().missing).toBe(true)
    expect(port.requests).toEqual([])
  })

  it('closes the document on the way home and forgets a deleted current file', async () => {
    const opening = library().openNew('report.pdf', new Uint8Array([1]))
    await answerOpening()
    await opening
    const [stored] = await documentLibrary.list()
    library().askToDelete(stored)
    await library().confirmDelete()
    expect(useDocumentStore.getState().document.phase).toBe('empty')
    expect(library()).toMatchObject({ files: [], current: null, deleting: null })
    await library().goHome()
    expect(useDocumentStore.getState().document.phase).toBe('empty')
  })

  it('keeps a reported problem until the next file opens', async () => {
    useToastStore.getState().show('library', 'Only PDF files can be opened.')
    const opening = library().openNew('report.pdf', new Uint8Array([1]))
    await answerOpening()
    await opening
    expect(useToastStore.getState().toasts.library).toBeUndefined()
  })
})
