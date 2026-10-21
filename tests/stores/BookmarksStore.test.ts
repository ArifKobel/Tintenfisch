import { describe, expect, it } from 'vitest'
import { EngineClient } from '@/services/EngineClient.ts'
import type { Bookmark, Request } from '@/types/protocol.ts'
import { BookmarksStore, UNTITLED } from '@/stores/BookmarksStore.ts'
import { TargetFollower } from '@/stores/TargetFollower.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'
import { ViewportStore } from '@/stores/ViewportStore.ts'

const bookmark = (id: string, title: string, children: Bookmark[] = [], open = false): Bookmark => ({
  id,
  title,
  target: { kind: 'page', pageId: '3-0', top: 20 },
  open,
  children,
})

const outline = [bookmark('11-0', 'Chapter 1', [bookmark('12-0', 'Section 1.1'), bookmark('13-0', 'Section 1.2')], true), bookmark('14-0', 'Chapter 2')]

async function createStore() {
  const port = new FakeEnginePort()
  const viewport = new ViewportStore()
  const store = new BookmarksStore(new EngineClient(port), new TargetFollower(viewport, () => {}))
  const loading = store.reload()
  await port.answerNext(outline)
  await loading
  return { port, viewport, store }
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('BookmarksStore', () => {
  it('follows a bookmark and keeps the expanded state the file gives until toggled', async () => {
    const { viewport, store } = await createStore()
    store.follow(outline[0].children[1])
    expect(store.getSnapshot().selected).toBe('13-0')
    expect(viewport.getSnapshot().focusRequest).toMatchObject({ pageId: '3-0', y: 20, atTop: true })
    expect(store.isCollapsed(outline[0])).toBe(false)
    store.toggleCollapsed(outline[0])
    expect(store.isCollapsed(outline[0])).toBe(true)
  })

  it('adds a bookmark after the selected one for the current view and starts renaming it', async () => {
    const { port, viewport, store } = await createStore()
    viewport.reportViewTop('4-0', 150)
    store.select('12-0')
    const adding = store.add()
    expect(last(port)).toMatchObject({ kind: 'addBookmark', parent: '11-0', index: 1, title: UNTITLED, target: { kind: 'page', pageId: '4-0', top: 150 } })
    await port.answerNext('20-0')
    await port.answerNext([bookmark('11-0', 'Chapter 1', [outline[0].children[0], bookmark('20-0', UNTITLED), outline[0].children[1]]), outline[1]])
    await adding
    expect(store.getSnapshot()).toMatchObject({ selected: '20-0', renaming: '20-0' })
    const renaming = store.rename('20-0', '  Overview ')
    expect(last(port)).toMatchObject({ kind: 'renameBookmark', bookmarkId: '20-0', title: 'Overview' })
    expect(store.getSnapshot().renaming).toBeNull()
    await port.answerNext(undefined)
    await port.answerNext(outline)
    await renaming
    const count = port.requests.length
    await store.rename('14-0', 'Chapter 2')
    await store.rename('14-0', '   ')
    expect(port.requests).toHaveLength(count)
  })

  it('moves a bookmark counting places without it, and skips moves that change nothing', async () => {
    const { port, store } = await createStore()
    const moving = store.move('12-0', { parent: '11-0', index: 2 })
    expect(last(port)).toMatchObject({ kind: 'moveBookmark', bookmarkId: '12-0', parent: '11-0', index: 1 })
    await port.answerNext(undefined)
    await port.answerNext(outline)
    await moving
    const count = port.requests.length
    await store.move('12-0', { parent: '11-0', index: 1 })
    await store.move('11-0', { parent: '11-0', index: 0 })
    expect(port.requests).toHaveLength(count)
    const nesting = store.move('14-0', { parent: '12-0', index: 0 })
    expect(store.isCollapsed(outline[0].children[0])).toBe(false)
    expect(last(port)).toMatchObject({ kind: 'moveBookmark', bookmarkId: '14-0', parent: '12-0', index: 0 })
    await port.answerNext(undefined)
    await port.answerNext(outline)
    await nesting
  })

  it('deletes the selected bookmark, retargets it to the view, and drops a selection that is gone', async () => {
    const { port, viewport, store } = await createStore()
    viewport.reportViewTop('5-0', 0)
    store.select('14-0')
    const retargeting = store.setDestination()
    expect(last(port)).toMatchObject({ kind: 'retargetBookmark', bookmarkId: '14-0', target: { kind: 'page', pageId: '5-0', top: 0 } })
    await port.answerNext(undefined)
    await port.answerNext(outline)
    await retargeting
    const removing = store.removeSelected()
    expect(last(port)).toMatchObject({ kind: 'deleteBookmark', bookmarkId: '14-0' })
    await port.answerNext(undefined)
    await port.answerNext([outline[0]])
    await removing
    expect(store.getSnapshot().selected).toBeNull()
  })
})
