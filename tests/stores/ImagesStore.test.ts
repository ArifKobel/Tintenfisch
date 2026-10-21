import { describe, expect, it } from 'vitest'
import { EngineClient } from '@/services/EngineClient.ts'
import type { Request } from '@/types/protocol.ts'
import { ImagesStore } from '@/stores/ImagesStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'
import { PageRevisionStore } from '@/stores/PageRevisionStore.ts'

const photo = { x: 40, y: 100, width: 80, height: 60, angle: 0 }
const turned = { x: 300, y: 50, width: 40, height: 20, angle: Math.PI / 2 }
const background = { x: 0, y: 0, width: 400, height: 400, angle: 0 }

function createStore() {
  const port = new FakeEnginePort()
  const revisions = new PageRevisionStore()
  const store = new ImagesStore(new EngineClient(port), revisions)
  return { port, revisions, store }
}

async function loaded(frames = [background, photo, turned]) {
  const created = createStore()
  const loading = created.store.load('3-0')
  await created.port.answerNext(frames)
  await loading
  return created
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('ImagesStore', () => {
  it('picks the topmost image under a point, turned frames included', async () => {
    const { store } = await loaded()
    store.selectAt('3-0', 60, 120)
    expect(store.getSnapshot().selected).toEqual({ pageId: '3-0', index: 1 })
    store.selectAt('3-0', 290, 70)
    expect(store.getSnapshot().selected).toEqual({ pageId: '3-0', index: 2 })
    store.selectAt('3-0', 310, 70)
    expect(store.getSnapshot().selected).toEqual({ pageId: '3-0', index: 0 })
    expect(store.selectedFrame).toEqual(background)
  })

  it('leaves images that cover half the page or more to the text above them', async () => {
    const { store } = await loaded()
    expect(store.foregroundAt('3-0', 60, 120, 400 * 400)).toBe(1)
    expect(store.foregroundAt('3-0', 10, 10, 400 * 400)).toBeNull()
  })

  it('moves an image, redraws its page and reads the frames again', async () => {
    const { port, store, revisions } = await loaded()
    const moved = { ...photo, x: 60 }
    const moving = store.move({ pageId: '3-0', index: 1 }, moved)
    expect(last(port)).toMatchObject({ kind: 'moveImage', pageId: '3-0', index: 1, frame: moved })
    await port.answerNext({ dirtyPages: ['3-0'] })
    expect(last(port)).toMatchObject({ kind: 'images', pageId: '3-0' })
    await port.answerNext([background, moved, turned])
    await moving
    expect(revisions.revisionOf('3-0')).toBe(1)
    expect(store.getSnapshot().pages['3-0'][1]).toEqual(moved)
  })

  it('selects an inserted image and drops the selection of a deleted one', async () => {
    const { port, store } = await loaded()
    const bytes = new Uint8Array([0x89, 0x50])
    const inserting = store.insert('3-0', bytes, 200, 210)
    expect(last(port)).toMatchObject({ kind: 'insertImage', pageId: '3-0', x: 200, y: 210 })
    await port.answerNext({ index: 3, dirtyPages: ['3-0'] })
    await port.answerNext([background, photo, turned, photo])
    await inserting
    expect(store.getSnapshot().selected).toEqual({ pageId: '3-0', index: 3 })
    const removing = store.removeSelected()
    expect(last(port)).toMatchObject({ kind: 'deleteImage', pageId: '3-0', index: 3 })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([background, photo, turned])
    await removing
    expect(store.getSnapshot().selected).toBeNull()
  })

  it('replaces the selected image and forgets a selection the page no longer has', async () => {
    const { port, store } = await loaded()
    store.select({ pageId: '3-0', index: 2 })
    const replacing = store.replaceSelected(new Uint8Array([0xff, 0xd8]))
    expect(last(port)).toMatchObject({ kind: 'replaceImage', pageId: '3-0', index: 2 })
    await port.answerNext({ dirtyPages: ['3-0'] })
    await port.answerNext([background, photo, turned])
    await replacing
    expect(store.getSnapshot().selected).toEqual({ pageId: '3-0', index: 2 })
    const reloading = store.load('3-0')
    await port.answerNext([background])
    await reloading
    expect(store.getSnapshot().selected).toBeNull()
  })

  it('shows what the engine refused', async () => {
    const { port, store } = await loaded()
    const inserting = store.insert('3-0', new Uint8Array([1]), 0, 0)
    await Promise.resolve()
    port.answer({ id: port.lastId, ok: false, error: 'only PNG and JPEG images can be placed' })
    await inserting
    expect(store.getSnapshot().notice).toBe('only PNG and JPEG images can be placed')
  })
})
