import { beforeEach, describe, expect, it, vi } from 'vitest'
import { pdfDate } from '@/lib/pdfDate.ts'
import type { Request } from '@/types/protocol.ts'
import { useCommentsStore } from '@/stores/commentsStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const comment = (id: string, contents = '') => ({
  id,
  pageId: '3-0',
  subtype: 'Text',
  contents,
  author: '',
  date: 'D:20261018120000Z',
  rect: { x: 10, y: 20, width: 20, height: 20 },
  color: [1, 0, 0] as [number, number, number],
})

function createStore() {
  const port = FakeEnginePort.attach()
  const tools = useWorkspaceStore.getState()
  const store = useCommentsStore.getState()
  return { port, tools, store }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(Date.UTC(2026, 9, 18, 12, 30, 5)))
})

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('commentsStore', () => {
  it('writes dates the way PDF does', () => {
    expect(pdfDate(new Date(Date.UTC(2026, 0, 2, 3, 4, 5)))).toBe('D:20260102030405Z')
  })

  it('adds a note in the chosen color, redraws its page, selects it and opens the panel', async () => {
    const { port, tools, store } = createStore()
    tools.choose('note')
    const adding = store.add('3-0', { kind: 'note', at: [10, 20], text: '' })
    expect(last(port)).toMatchObject({
      kind: 'addMarkup',
      pageId: '3-0',
      markup: { kind: 'note' },
      style: { color: useWorkspaceStore.getState().color, width: 2, date: 'D:20261018123005Z' },
    })
    await port.answerNext({ id: '20-0', dirtyPages: ['3-0'] })
    await port.answerNext([comment('20-0')])
    await adding
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
    expect(useCommentsStore.getState()).toMatchObject({ selected: '20-0' })
    expect(useWorkspaceStore.getState().right).toBe('comments')
  })

  it('changes and deletes comments, dropping a selection that is gone', async () => {
    const { port, store } = createStore()
    const loading = store.reload()
    await port.answerNext([comment('20-0', 'old')])
    await loading
    store.select('20-0')
    const editing = store.setContents('20-0', 'new')
    expect(last(port)).toMatchObject({ kind: 'setCommentContents', commentId: '20-0', contents: 'new' })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([comment('20-0', 'new')])
    await editing
    expect(useDocumentStore.getState().pageRevisions['3-0']).toBe(1)
    await store.setContents('20-0', 'new')
    expect(port.requests).toHaveLength(3)
    const removing = store.removeSelected()
    expect(last(port)).toMatchObject({ kind: 'deleteComment', pageId: '3-0', commentId: '20-0' })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([])
    await removing
    expect(useCommentsStore.getState()).toMatchObject({ comments: [], selected: null })
  })
})
