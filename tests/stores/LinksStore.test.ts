import { describe, expect, it } from 'vitest'
import { EngineClient } from '@/services/EngineClient.ts'
import type { DocumentLink, Request } from '@/types/protocol.ts'
import { LinksStore } from '@/stores/LinksStore.ts'
import { TargetFollower } from '@/stores/TargetFollower.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'
import { PageRevisionStore } from '@/stores/PageRevisionStore.ts'
import { ViewportStore } from '@/stores/ViewportStore.ts'

const toPage: DocumentLink = { id: '20-0', rect: { x: 28, y: 96, width: 132, height: 18 }, target: { kind: 'page', pageId: '5-0', top: 100 } }
const toWeb: DocumentLink = { id: '21-0', rect: { x: 28, y: 126, width: 132, height: 18 }, target: { kind: 'uri', uri: 'https://example.org/' } }

async function createStore() {
  const port = new FakeEnginePort()
  const viewport = new ViewportStore()
  const revisions = new PageRevisionStore()
  const opened: string[] = []
  const store = new LinksStore(new EngineClient(port), revisions, new TargetFollower(viewport, (uri) => opened.push(uri)))
  const loading = store.load('3-0')
  await port.answerNext([toPage, toWeb])
  await loading
  return { port, viewport, revisions, store, opened }
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('LinksStore', () => {
  it('follows links to pages and web addresses', async () => {
    const { viewport, store, opened } = await createStore()
    store.follow(toPage)
    expect(viewport.getSnapshot().focusRequest).toMatchObject({ pageId: '5-0', y: 100, atTop: true })
    store.follow(toWeb)
    expect(opened).toEqual(['https://example.org/'])
  })

  it('adds a drawn link once it has a target and selects it', async () => {
    const { port, store, revisions } = await createStore()
    const rect = { x: 10, y: 20, width: 100, height: 30 }
    store.draw('3-0', rect)
    expect(store.getSnapshot().draft).toEqual({ pageId: '3-0', linkId: null, rect, target: null })
    const saving = store.save({ kind: 'uri', uri: 'https://example.org/new' })
    expect(last(port)).toMatchObject({ kind: 'addLink', pageId: '3-0', rect, target: { kind: 'uri', uri: 'https://example.org/new' } })
    await port.answerNext({ id: '22-0', dirtyPages: ['3-0'] })
    await port.answerNext([toPage, toWeb, { id: '22-0', rect, target: { kind: 'uri', uri: 'https://example.org/new' } }])
    await saving
    expect(store.getSnapshot()).toMatchObject({ draft: null, selected: { pageId: '3-0', id: '22-0' } })
    expect(revisions.revisionOf('3-0')).toBe(1)
  })

  it('edits, moves and deletes the selected link', async () => {
    const { port, store } = await createStore()
    store.select({ pageId: '3-0', id: '20-0' })
    store.editSelected()
    expect(store.getSnapshot().draft).toEqual({ pageId: '3-0', linkId: '20-0', rect: toPage.rect, target: toPage.target })
    const saving = store.save({ kind: 'page', pageId: '4-0', top: null })
    expect(last(port)).toMatchObject({ kind: 'updateLink', linkId: '20-0', rect: toPage.rect, target: { kind: 'page', pageId: '4-0', top: null } })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([toPage, toWeb])
    await saving
    const moved = { ...toPage.rect, x: 40 }
    const moving = store.move({ pageId: '3-0', id: '20-0' }, moved)
    expect(last(port)).toMatchObject({ kind: 'updateLink', linkId: '20-0', rect: moved, target: toPage.target })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([toPage, toWeb])
    await moving
    const removing = store.removeSelected()
    expect(last(port)).toMatchObject({ kind: 'deleteComment', pageId: '3-0', commentId: '20-0' })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([toWeb])
    await removing
    expect(store.getSnapshot().selected).toBeNull()
  })
})
