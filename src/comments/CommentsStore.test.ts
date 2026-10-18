import { describe, expect, it } from 'vitest'
import { CommentsStore } from '@comments/CommentsStore.ts'
import { EngineClient } from '@engine/EngineClient.ts'
import type { Request } from '@engine/protocol.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'
import { ToolStore } from '@tools/ToolStore.ts'
import { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

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
  const port = new FakeEnginePort()
  const tools = new ToolStore()
  const revisions = new PageRevisionStore()
  const store = new CommentsStore(new EngineClient(port), tools, revisions, () => new Date(Date.UTC(2026, 9, 18, 12, 30, 5)))
  return { port, tools, revisions, store }
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('CommentsStore', () => {
  it('writes dates the way PDF does', () => {
    expect(CommentsStore.pdfDate(new Date(Date.UTC(2026, 0, 2, 3, 4, 5)))).toBe('D:20260102030405Z')
  })

  it('adds a note in the chosen color, redraws its page, selects it and opens the panel', async () => {
    const { port, tools, revisions, store } = createStore()
    tools.choose('note')
    const adding = store.add('3-0', { kind: 'note', at: [10, 20], text: '' })
    expect(last(port)).toMatchObject({
      kind: 'addMarkup',
      pageId: '3-0',
      markup: { kind: 'note' },
      style: { color: tools.getSnapshot().color, width: 2, date: 'D:20261018123005Z' },
    })
    await port.answerNext({ id: '20-0', dirtyPages: ['3-0'] })
    await port.answerNext([comment('20-0')])
    await adding
    expect(revisions.revisionOf('3-0')).toBe(1)
    expect(store.getSnapshot()).toMatchObject({ selected: '20-0', open: true })
  })

  it('changes and deletes comments, dropping a selection that is gone', async () => {
    const { port, store, revisions } = createStore()
    const loading = store.reload()
    await port.answerNext([comment('20-0', 'old')])
    await loading
    store.select('20-0')
    const editing = store.setContents('20-0', 'new')
    expect(last(port)).toMatchObject({ kind: 'setCommentContents', commentId: '20-0', contents: 'new' })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([comment('20-0', 'new')])
    await editing
    expect(revisions.revisionOf('3-0')).toBe(1)
    await store.setContents('20-0', 'new')
    expect(port.requests).toHaveLength(3)
    const removing = store.removeSelected()
    expect(last(port)).toMatchObject({ kind: 'deleteComment', pageId: '3-0', commentId: '20-0' })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([])
    await removing
    expect(store.getSnapshot()).toMatchObject({ comments: [], selected: null })
  })
})
