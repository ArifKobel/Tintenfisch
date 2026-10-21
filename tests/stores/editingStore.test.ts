import { describe, expect, it } from 'vitest'
import type { Request } from '@/types/protocol.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useEditingStore } from '@/stores/editingStore.ts'
import { useToastStore } from '@/stores/toastStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const caret = (x: number) => ({ x, y: 10, width: 0, height: 12, angle: 0 })
const frame = { x: 5, y: 8, width: 300, height: 40, angle: 0 }
const preview = (x: number) => ({ caret: caret(x), frame, selection: [] })
const fonts = [{ name: 'F1', font: 'Helvetica', baseFont: 'Helvetica', embedded: false, family: 'sans' as const, bold: false, italic: false, weight: null }]
const look = { font: { kind: 'document', name: 'F1' }, size: 12, bold: false, italic: false, color: [0, 0, 0] }
const styles = (count: number) => Array.from({ length: count }, () => ({ resource: 'F1', font: 'Helvetica', size: 12, bold: false, italic: false, color: [0, 0, 0] }))

function createStore() {
  const port = FakeEnginePort.attach()
  const store = useEditingStore.getState()
  store.reset()
  store.setEnabled(true)
  return { port, store }
}

async function open(port: FakeEnginePort) {
  const begin = useEditingStore.getState().begin('3-0', 40, 15)
  await port.answerNext({ line: 1, offset: 2 })
  await port.answerNext({ line: 1, clusters: ['a', 'b', 'c'], start: 0, styles: styles(3) })
  await port.answerNext(preview(20))
  await port.answerNext(fonts)
  await begin
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('editingStore', () => {
  it('types new text at a click and commits it without a caret to reopen', async () => {
    const { port, store } = createStore()
    const begin = store.beginNew('3-0', 120, 80)
    expect(last(port)).toMatchObject({ kind: 'previewNewText', pageId: '3-0', draft: { x: 120, y: 80, size: 12, family: 'sans', text: '', caret: 0 } })
    await port.answerNext(preview(120))
    await port.answerNext(fonts)
    await begin
    store.edit((session) => session.insert('Hi'))
    expect(last(port)).toMatchObject({ kind: 'previewNewText', draft: { x: 120, y: 80, text: 'Hi', caret: 2 } })
    await port.answerNext(preview(134))
    await Promise.resolve()
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
    const committing = store.commit()
    expect(last(port)).toMatchObject({ kind: 'commitEdit', uiState: '' })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await committing
    expect(useEditingStore.getState().active).toBeNull()
  })

  it('opens the line under the click with the caret where it landed', async () => {
    const { port } = createStore()
    await open(port)
    expect(port.requests.map((request) => request.kind)).toEqual(['hitText', 'paragraphText', 'previewEdit', 'pageFonts'])
    expect(port.requests[2]).toMatchObject({ pageId: '3-0', line: 1, draft: { clusters: ['a', 'b', 'c'], caret: 2 } })
    expect(useEditingStore.getState()).toMatchObject({ active: { pageId: '3-0', caret: caret(20), frame, selection: [], look }, fonts })
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBeUndefined()
  })

  it('edits the whole paragraph of a click on its second line', async () => {
    const { port, store } = createStore()
    const begin = store.begin('3-0', 40, 30)
    await port.answerNext({ line: 5, offset: 1 })
    expect(last(port)).toMatchObject({ kind: 'paragraphText', line: 5 })
    await port.answerNext({ line: 4, clusters: ['a', 'b', ' ', 'c', 'd'], start: 3, styles: styles(5) })
    expect(last(port)).toMatchObject({ kind: 'previewEdit', line: 4, draft: { clusters: ['a', 'b', ' ', 'c', 'd'], caret: 4 } })
    await port.answerNext(preview(30))
    await port.answerNext(fonts)
    await begin
    store.edit((session) => session.insert('\n'))
    expect(last(port)).toMatchObject({ kind: 'previewEdit', line: 4, draft: { clusters: ['a', 'b', ' ', 'c', '\n', 'd'], caret: 5 } })
  })

  it('previews every keystroke and redraws the page only when the text changes', async () => {
    const { port, store } = createStore()
    await open(port)
    store.edit((session) => session.insert('X'))
    expect(last(port)).toMatchObject({ kind: 'previewEdit', draft: { clusters: ['a', 'b', 'X', 'c'], caret: 3 }, group: 'edit-preview' })
    await port.answerNext(preview(26))
    await Promise.resolve()
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
    store.edit((session) => session.moveBy(-1))
    await port.answerNext(preview(20))
    await Promise.resolve()
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
    expect(useEditingStore.getState().active?.caret).toEqual(caret(20))
  })

  it('previews a moved and resized box and redraws the page for it', async () => {
    const { port, store } = createStore()
    await open(port)
    store.edit((session) => session.movedTo(12, -4).resizedTo(220))
    expect(last(port)).toMatchObject({ kind: 'previewEdit', draft: { clusters: ['a', 'b', 'c'], dx: 12, dy: -4, width: 220 } })
    await port.answerNext(preview(20))
    await Promise.resolve()
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
    const commit = store.commit()
    expect(last(port).kind).toBe('commitEdit')
    await port.answerNext({ dirtyPages: ['3-0'] })
    await commit
  })

  it('falls back to the last accepted draft when the engine refuses a keystroke', async () => {
    const { port, store } = createStore()
    await open(port)
    store.edit((session) => session.insert('\u{1F600}'))
    await Promise.resolve()
    port.answer({ id: port.lastId, ok: false, error: 'no font can show it' })
    await Promise.resolve()
    await Promise.resolve()
    expect(useToastStore.getState().toasts.editing).toBe('no font can show it')
    expect(last(port)).toMatchObject({ kind: 'previewEdit', draft: { clusters: ['a', 'b', 'c'] } })
  })

  it('sends a selection and the styles laid over it, and shows what the selection looks like', async () => {
    const { port, store } = createStore()
    await open(port)
    store.edit((session) => session.moveTo(0).extendBy(2))
    expect(last(port)).toMatchObject({ kind: 'previewEdit', draft: { clusters: ['a', 'b', 'c'], selection: [0, 2], styles: [] } })
    const boxes = [{ x: 5, y: 10, width: 14, height: 12, angle: 0 }]
    await port.answerNext({ ...preview(19), selection: boxes })
    await Promise.resolve()
    expect(useEditingStore.getState().active?.selection).toEqual(boxes)
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBeUndefined()
    store.edit((session) => session.formatted({ bold: true, color: [1, 0, 0] }))
    expect(last(port)).toMatchObject({ draft: { styles: [{ start: 0, end: 2, bold: true, color: [1, 0, 0] }], selection: [0, 2] } })
    await port.answerNext({ ...preview(21), selection: boxes })
    await Promise.resolve()
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
    expect(useEditingStore.getState().active?.look).toMatchObject({ bold: true, color: [1, 0, 0] })
    expect(store.session()?.changed).toBe(true)
  })

  it('sets new text in the face, size and color chosen for it', async () => {
    const { port, store } = createStore()
    const begin = store.beginNew('3-0', 120, 80)
    await port.answerNext(preview(120))
    await port.answerNext(fonts)
    await begin
    store.edit((session) => session.formatted({ font: { kind: 'bundled', family: 'serif' }, italic: true, size: 18, color: [0, 0, 1] }))
    expect(last(port)).toMatchObject({ kind: 'previewNewText', draft: { family: 'serif', bold: false, italic: true, size: 18, color: [0, 0, 1] } })
  })

  it('commits a changed line with its caret as the undo state', async () => {
    const { port, store } = createStore()
    await open(port)
    store.edit((session) => session.insert('X'))
    await port.answerNext(preview(26))
    const commit = store.commit()
    expect(last(port)).toMatchObject({ kind: 'commitEdit' })
    expect(JSON.parse((last(port) as Extract<Request, { kind: 'commitEdit' }>).uiState)).toEqual({ pageId: '3-0', line: 1, offset: 3 })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await commit
    expect(useEditingStore.getState().active).toBeNull()
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(2)
  })

  it('cancels instead of committing a line that reads as before', async () => {
    const { port, store } = createStore()
    await open(port)
    const commit = store.commit()
    expect(last(port).kind).toBe('cancelEdit')
    await port.answerNext(undefined)
    await commit
  })

  it('undo first discards an unsaved draft, then reverts the document and reopens the caret', async () => {
    const { port, store } = createStore()
    await open(port)
    store.edit((session) => session.deleteBackward())
    await port.answerNext(preview(14))
    const discard = store.undo()
    expect(last(port).kind).toBe('cancelEdit')
    await port.answerNext(undefined)
    await discard
    expect(useEditingStore.getState().active).toBeNull()
    const undo = store.undo()
    expect(last(port).kind).toBe('undo')
    await port.answerNext({ dirtyPages: ['3-0'], uiState: JSON.stringify({ pageId: '3-0', line: 1, offset: 3 }) })
    await port.answerNext({ line: 1, clusters: ['a', 'b', 'c', 'd'], start: 2, styles: styles(4) })
    await port.answerNext(preview(30))
    expect(last(port)).toMatchObject({ kind: 'previewEdit', draft: { clusters: ['a', 'b', 'c', 'd'], caret: 3 } })
    await port.answerNext(fonts)
    await undo
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(3)
    expect(useEditingStore.getState().active).toMatchObject({ pageId: '3-0', caret: caret(30), frame })
  })
})
